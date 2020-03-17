use futures::prelude::*;

use libp2p_swarm::{
    KeepAlive, ProtocolsHandler, ProtocolsHandlerEvent, ProtocolsHandlerUpgrErr, SubstreamProtocol,
};
use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
};
use void::Void;

use crate::{
    Announce, AnnounceFailure, AnnounceMessage, SwapDigest, SwapId,
};
use std::time::Duration;
use wasm_timer::Delay;

pub struct AnnounceHandler {
    /// The pending results from inbound or outbound pings, ready
    /// to be `poll()`ed.
    pending_results: VecDeque<AnnounceMessage>,
    state: AnnounceState,
}

pub enum AnnounceState {
    Started,
    Announced(SwapDigest),
    Confirmed(SwapDigest, SwapId),
}

impl AnnounceHandler {
    pub fn new() -> Self {
        AnnounceHandler {
            pending_results: VecDeque::with_capacity(2),
            state: AnnounceState::Started,
        }
    }
}

impl ProtocolsHandler for AnnounceHandler {
    type InEvent = SwapDigest;
    type OutEvent = SwapId;
    type Error = AnnounceFailure;
    type InboundProtocol = Announce;
    type OutboundProtocol = Announce;
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Announce> {
        SubstreamProtocol::new(Announce::new())
    }

    /// Injects/registers? succesfully upgraded inbound connection?
    fn inject_fully_negotiated_inbound(&mut self, rtt: SwapDigest) {
        // An announcement remote peer has been confirmed.
        self.pending_results
            .push_front(AnnounceMessage::Digest(rtt));
    }

    /// Injects/registers? succesfully upgraded outbound connection?
    fn inject_fully_negotiated_outbound(&mut self, rtt: SwapId, _info: ()) {
        // An announcement initiated by the local peer was confirmed by the remote.
        self.pending_results
            .push_front(AnnounceMessage::Id(rtt));
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

    /// kill connection if confirmation received ie. announce protocol has completed
    fn connection_keep_alive(&self) -> KeepAlive {
        match self.state {
            AnnounceState::Started => KeepAlive::Yes,
            AnnounceState::Announced(_) => KeepAlive::Yes,
            AnnounceState::Confirmed(_,_) => KeepAlive::No,
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context,
    ) -> Poll<ProtocolsHandlerEvent<Announce, (), , Self::Error>> {
        match self.pending_results.pop_back() {
            AnnounceMessage::Digest(digest) => {
                match self.state {
                    AnnounceState::Announced(digest) => self.state = AnnounceState::Confirmed(digest, id),
                    _ => ()
                }
            }
            AnnounceMessage::Id(id)) = {
                match self.state {
                    AnnounceState::Started => self.state = AnnounceState::Announced(digest),
                    _ => ()
                }
            }
        }
    }
}
