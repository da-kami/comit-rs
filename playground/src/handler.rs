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
    Announce, AnnounceFailure, AnnounceResult, AnnounceSuccess, Announcement, Confirmation,
};
use std::time::Duration;
use wasm_timer::Delay;

pub struct AnnounceHandler {
    /// The pending results from inbound or outbound pings, ready
    /// to be `poll()`ed.
    pending_results: VecDeque<AnnounceResult>,
}

impl AnnounceHandler {
    pub fn new() -> Self {
        AnnounceHandler {
            pending_results: VecDeque::with_capacity(2),
        }
    }
}

impl ProtocolsHandler for AnnounceHandler {
    type InEvent = Announcement;
    type OutEvent = Confirmation;
    type Error = AnnounceFailure;
    type InboundProtocol = Announce;
    type OutboundProtocol = Announce;
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Announce> {
        SubstreamProtocol::new(Announce::new())
    }

    /// Injects/registers? upgraded succesfully upgraded inbound
    fn inject_fully_negotiated_inbound(&mut self, _rtt: Confirmation) {
        // An announcement remote peer has been confirmed.
        self.pending_results
            .push_front(Ok(AnnounceSuccess::Announce));
    }

    /// Injects/registers? succesfully upgraded outbound substream
    fn inject_fully_negotiated_outbound(&mut self, _rtt: Announcement, _info: ()) {
        // An announcement initiated by the local peer was confirmed by the remote.
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
                // self.failures = 0;
            }
            if let Err(e) = result {
                return Poll::Ready(ProtocolsHandlerEvent::Custom(Err(e)));
            }
            return Poll::Ready(ProtocolsHandlerEvent::Custom(result));
        }

        // match Future::poll(Pin::new(&mut self.next_ping), cx) {
        match Future::poll(Pin::new(&mut self), cx) {
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
