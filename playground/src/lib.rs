use futures::prelude::*;
use libp2p::PeerId;
use libp2p_core::{ConnectedPoint, InboundUpgrade, Multiaddr, OutboundUpgrade, UpgradeInfo};
use libp2p_swarm::{
    KeepAlive, NetworkBehaviour, NetworkBehaviourAction, PollParameters, ProtocolsHandler,
    ProtocolsHandlerEvent, ProtocolsHandlerUpgrErr, SubstreamProtocol,
};
use std::{
    collections::VecDeque,
    iter,
    pin::Pin,
    task::{Context, Poll},
};
use void::Void;
// use thiserror::Error;
use futures::future::BoxFuture;
use rand::{distributions, prelude::*};
use std::{fmt, io, time::Duration};
use wasm_timer::{Delay, Instant};

const PING_SIZE: usize = 32;

pub struct AnnounceBehaviour {
    config: AnnounceConfig,
    events: VecDeque<AnnounceEvent>,
}

impl UpgradeInfo for AnnounceBehaviour {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/comit/swap/announce/1.0.0")
    }
}

impl<TSocket> InboundUpgrade<TSocket> for AnnounceBehaviour
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = ();
    type Error = io::Error;
    type Future = BoxFuture<'static, Result<(), io::Error>>;

    fn upgrade_inbound(self, mut socket: TSocket, _: Self::Info) -> Self::Future {
        async move {
            let mut payload = [0u8; PING_SIZE];
            while let Ok(_) = socket.read_exact(&mut payload).await {
                socket.write_all(&payload).await?;
            }
            Ok(())
        }
        .boxed()
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for AnnounceBehaviour
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = Duration;
    type Error = std::io::Error;
    type Future = BoxFuture<'static, Result<Duration, io::Error>>;

    fn upgrade_outbound(self, mut socket: TSocket, _: Self::Info) -> Self::Future {
        let payload: [u8; 32] = thread_rng().sample(distributions::Standard);
        // debug!("Preparing ping payload {:?}", payload);
        async move {
            socket.write_all(&payload).await?;
            socket.close().await?;
            let started = Instant::now();

            let mut recv_payload = [0u8; 32];
            socket.read_exact(&mut recv_payload).await?;
            if recv_payload == payload {
                Ok(started.elapsed())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Ping payload mismatch",
                ))
            }
        }
        .boxed()
    }
}

impl AnnounceBehaviour {
    pub fn new(config: AnnounceConfig) -> Self {
        AnnounceBehaviour {
            events: VecDeque::new(),
            config,
        }
    }
}

/// The result of an inbound or outbound ping.
pub type AnnounceResult = Result<AnnounceSuccess, AnnounceFailure>;

/// The successful result of processing an inbound or outbound ping.
#[derive(Debug)]
pub enum AnnounceSuccess {
    /// Received an announce and sent back a confirmation.
    Confirmation,
    /// Send and announce
    Announce,
}

/// An outbound announce failure.
#[derive(Debug)]
pub enum AnnounceFailure {
    /// The announce timed out, i.e. no response was received within the
    /// configured announce timeout.
    //#[error("Failed due to timeout")]
    Timeout,
    /// The announce failed for reasons other than a timeout.
    //#[error("Ping failed because")]
    Other,
}

impl std::fmt::Display for AnnounceFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AnnounceFailure::Timeout => f.write_str("Ping timeout"),
            AnnounceFailure::Other => write!(f, "Ping error"),
        }
    }
}

impl std::error::Error for AnnounceFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AnnounceFailure::Timeout => None,
            AnnounceFailure::Other => None,
        }
    }
}

#[derive(Debug)]
pub struct AnnounceEvent;

/// The configuration for outbound pings.
#[derive(Clone, Debug, Default)]
pub struct AnnounceConfig {
    /// The timeout of an outbound ping.
    timeout: Duration,
    /// The maximum number of failed outbound announce before the associated
    /// connection is deemed unhealthy, indicating to the `Swarm` that it
    /// should be closed.
    max_failures: u32,
    /// Whether the connection should generally be kept alive unless
    /// `max_failures` occur.
    keep_alive: bool,
}

impl AnnounceConfig {
    /// Creates a new `AnnounceConfig` with the following default settings:
    ///
    ///   * [`AnnounceConfig::with_interval`] 15s
    ///   * [`AnnounceConfig::with_timeout`] 20s
    ///   * [`AnnounceConfig::with_max_failures`] 1
    ///   * [`AnnounceConfig::with_keep_alive`] false
    ///
    /// These settings have the following effect:
    ///
    ///   * A ping is sent every 15 seconds on a healthy connection.
    ///   * Every ping sent must yield a response within 20 seconds in order to
    ///     be successful.
    ///   * A single ping failure is sufficient for the connection to be subject
    ///     to being closed.
    ///   * The connection may be closed at any time as far as the ping protocol
    ///     is concerned, i.e. the ping protocol itself does not keep the
    ///     connection alive.
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(20),
            max_failures: 1,
            keep_alive: false,
        }
    }

    /// Sets the ping timeout.
    pub fn with_timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }

    /// Sets the maximum number of consecutive ping failures upon which the
    /// remote peer is considered unreachable and the connection closed.
    pub fn with_max_failures(mut self, n: u32) -> Self {
        self.max_failures = n;
        self
    }

    /// Sets whether the ping protocol itself should keep the connection alive,
    /// apart from the maximum allowed failures.
    ///
    /// By default, the ping protocol itself allows the connection to be closed
    /// at any time, i.e. in the absence of ping failures the connection
    /// lifetime is determined by other protocol handlers.
    ///
    /// If the maximum number of allowed ping failures is reached, the
    /// connection is always terminated as a result of
    /// [`ProtocolsHandler::poll`] returning an error, regardless of the
    /// keep-alive setting.
    pub fn with_keep_alive(mut self, b: bool) -> Self {
        self.keep_alive = b;
        self
    }
}

pub struct AnnounceHandler {
    /// Configuration options.
    config: AnnounceConfig,
    /// The timer for when to send the next ping.
    next_ping: Delay,
    /// The pending results from inbound or outbound pings, ready
    /// to be `poll()`ed.
    pending_results: VecDeque<AnnounceResult>,
    /// The number of consecutive ping failures that occurred.
    failures: u32,
}

impl AnnounceHandler {
    pub fn new(config: AnnounceConfig) -> Self {
        AnnounceHandler {
            config,
            next_ping: Delay::new(Duration::new(0, 0)),
            pending_results: VecDeque::with_capacity(2),
            failures: 0,
        }
    }
}

impl ProtocolsHandler for AnnounceHandler {
    type InEvent = Void;
    type OutEvent = AnnounceResult;
    type Error = AnnounceFailure;
    type InboundProtocol = AnnounceBehaviour;
    type OutboundProtocol = AnnounceBehaviour;
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<AnnounceBehaviour> {
        SubstreamProtocol::new(AnnounceBehaviour::new(self.config.clone()))
    }

    fn inject_fully_negotiated_inbound(&mut self, _: ()) {
        // A ping from a remote peer has been answered.
        self.pending_results
            .push_front(Ok(AnnounceSuccess::Announce));
    }

    fn inject_fully_negotiated_outbound(&mut self, _rtt: std::time::Duration, _info: ()) {
        // A ping initiated by the local peer was answered by the remote.
        self.pending_results
            .push_front(Ok(AnnounceSuccess::Confirmation));
    }

    fn inject_event(&mut self, _: Void) {}

    fn inject_dial_upgrade_error(
        &mut self,
        _info: (),
        error: ProtocolsHandlerUpgrErr<std::io::Error>,
    ) {
        self.pending_results.push_front(Err(match error {
            ProtocolsHandlerUpgrErr::Timeout => AnnounceFailure::Timeout,
            ProtocolsHandlerUpgrErr::Upgrade(_) => AnnounceFailure::Timeout,
            _ => AnnounceFailure::Timeout,
        }))
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        if self.config.keep_alive {
            KeepAlive::Yes
        } else {
            KeepAlive::No
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context,
    ) -> Poll<ProtocolsHandlerEvent<AnnounceBehaviour, (), AnnounceResult, Self::Error>> {
        if let Some(result) = self.pending_results.pop_back() {
            if let Ok(AnnounceSuccess::Announce) = result {
                self.failures = 0;
                // self.next_ping.reset(self.config.interval);
            }
            if let Err(e) = result {
                self.failures += 1;
                if self.failures >= self.config.max_failures {
                    return Poll::Ready(ProtocolsHandlerEvent::Close(e));
                } else {
                    return Poll::Ready(ProtocolsHandlerEvent::Custom(Err(e)));
                }
            }
            return Poll::Ready(ProtocolsHandlerEvent::Custom(result));
        }

        match Future::poll(Pin::new(&mut self.next_ping), cx) {
            Poll::Ready(Ok(())) => {
                self.next_ping.reset(self.config.timeout);
                let protocol = SubstreamProtocol::new(AnnounceBehaviour::new(self.config.clone()))
                    .with_timeout(self.config.timeout);
                Poll::Ready(ProtocolsHandlerEvent::OutboundSubstreamRequest { protocol, info: () })
            }
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(_)) => {
                Poll::Ready(ProtocolsHandlerEvent::Close(AnnounceFailure::Timeout))
            }
        }
    }
}

impl NetworkBehaviour for AnnounceBehaviour {
    type ProtocolsHandler = AnnounceHandler;
    type OutEvent = AnnounceEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        AnnounceHandler::new(self.config.clone())
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, _peer_id: PeerId, _endpoint: ConnectedPoint) {
        unimplemented!()
    }

    fn inject_disconnected(&mut self, _peer_id: &PeerId, _endpoint: ConnectedPoint) {
        unimplemented!()
    }

    fn inject_node_event(&mut self, _peer_id: PeerId, _result: AnnounceResult) {
        unimplemented!()
    }

    // fn poll(&mut self, cx: &mut Context,
    //         params: &mut impl PollParameters<SupportedProtocolsIter=_,
    //             ListenedAddressesIter=_, ExternalAddressesIter=_>)
    //     -> Poll<NetworkBehaviourAction<_, Self::OutEvent>> {
    //     unimplemented!()
    // }

    fn poll(
        &mut self,
        _: &mut Context,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Void, AnnounceEvent>> {
        if let Some(e) = self.events.pop_back() {
            Poll::Ready(NetworkBehaviourAction::GenerateEvent(e))
        } else {
            Poll::Pending
        }
    }
}
