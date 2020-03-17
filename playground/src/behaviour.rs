use futures::prelude::*;
use libp2p::PeerId;
use libp2p_core::{ConnectedPoint, InboundUpgrade, Multiaddr, OutboundUpgrade, UpgradeInfo};
use libp2p_swarm::{NetworkBehaviour, NetworkBehaviourAction, PollParameters};
use std::{
    collections::VecDeque,
    iter,
    task::{Context, Poll},
};
use void::Void;
// use thiserror::Error;
use crate::{handler::AnnounceHandler, AnnounceEvent, AnnounceResult, PING_SIZE};
use futures::future::BoxFuture;
use rand::{distributions, prelude::*};
use std::{io, time::Duration};
use wasm_timer::Instant;

pub struct Announce {
    events: VecDeque<AnnounceEvent>,
}

impl Announce {
    pub fn new() -> Self {
        Announce {
            events: VecDeque::new(),
        }
    }
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
    type Output = Announcement;
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
    type Output = Confirmation;
    type Error = std::io::Error;
    type Future = BoxFuture<'static, Result<(), io::Error>>;

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
                // Ok(started.elapsed())
                Ok(())
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

impl NetworkBehaviour for Announce {
    type ProtocolsHandler = AnnounceHandler;
    type OutEvent = AnnounceEvent;

    /// create a new protocols handler for each incoming connection or each time
    /// we dial a node
    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        AnnounceHandler::new()
    }

    /// returns a list of addresses ordered by reachability
    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    ///
    fn inject_connected(&mut self, _peer_id: PeerId, _endpoint: ConnectedPoint) {}

    fn inject_disconnected(&mut self, _peer_id: &PeerId, _endpoint: ConnectedPoint) {}

    /// Pushes protocol handler events to the network behaviour
    fn inject_node_event(&mut self, peer: PeerId, result: AnnounceResult) {
        self.events.push_front(AnnounceEvent { peer, result })
    }

    /// handle/consume? events
    fn poll(
        &mut self,
        _: &mut Context,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Void, AnnounceEvent>> {
        if let Some(e) = self.events.pop_back() {
            Poll::Ready(NetworkBehaviourAction::GenerateEvent(e))
        // Poll::Pending
        } else {
            Poll::Pending
        }
    }
}
