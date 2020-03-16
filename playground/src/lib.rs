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

pub struct Announce {
    events: VecDeque<AnnounceEvent>,
}

impl UpgradeInfo for Announce {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/comit/swap/announce/1.0.0")
    }
}

impl<TSocket> InboundUpgrade<TSocket> for Announce
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

impl<TSocket> OutboundUpgrade<TSocket> for Announce
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

impl Announce {
    pub fn new() -> Self {
        Announce {
            events: VecDeque::new(),
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
pub struct AnnounceEvent {
    /// The peer ID of the remote.
    pub peer: PeerId,
    /// The result of an inbound or outbound ping.
    pub result: AnnounceResult,
}

pub struct AnnounceHandler {
    /// Configuration options.
    /// The timer for when to send the next ping.
    next_ping: Delay,
    /// The pending results from inbound or outbound pings, ready
    /// to be `poll()`ed.
    pending_results: VecDeque<AnnounceResult>,
    /// The number of consecutive ping failures that occurred.
    failures: u32,
}

impl AnnounceHandler {
    pub fn new() -> Self {
        AnnounceHandler {
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
    type InboundProtocol = Announce;
    type OutboundProtocol = Announce;
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Announce> {
        SubstreamProtocol::new(Announce::new())
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
        KeepAlive::Yes
    }

    fn poll(
        &mut self,
        cx: &mut Context,
    ) -> Poll<ProtocolsHandlerEvent<Announce, (), AnnounceResult, Self::Error>> {
        if let Some(result) = self.pending_results.pop_back() {
            if let Ok(AnnounceSuccess::Announce) = result {
                self.failures = 0;
            }
            if let Err(e) = result {
                return Poll::Ready(ProtocolsHandlerEvent::Custom(Err(e)));
            }
            return Poll::Ready(ProtocolsHandlerEvent::Custom(result));
        }

        match Future::poll(Pin::new(&mut self.next_ping), cx) {
            Poll::Ready(Ok(())) => {
                let protocol = SubstreamProtocol::new(Announce::new());
                Poll::Ready(ProtocolsHandlerEvent::OutboundSubstreamRequest { protocol, info: () })
            }
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(_)) => {
                Poll::Ready(ProtocolsHandlerEvent::Close(AnnounceFailure::Timeout))
            }
        }
    }
}

impl NetworkBehaviour for Announce {
    type ProtocolsHandler = AnnounceHandler;
    type OutEvent = AnnounceEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        AnnounceHandler::new()
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, _peer_id: PeerId, _endpoint: ConnectedPoint) {}

    fn inject_disconnected(&mut self, _peer_id: &PeerId, _endpoint: ConnectedPoint) {}

    fn inject_node_event(&mut self, peer: PeerId, result: AnnounceResult) {
        self.events.push_front(AnnounceEvent { peer, result })
    }

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
