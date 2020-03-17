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

use crate::{protocols::StateMachine, Announce, AnnounceFailure, SwapDigest, SwapId};
use libp2p_swarm::protocols_handler::InboundUpgradeSend;
use std::{collections::HashMap, time::Duration};
use wasm_timer::Delay;

pub struct AnnounceHandler {
    /// The pending results from inbound or outbound pings, ready
    /// to be `poll()`ed.
    events_out: VecDeque<StateMachine>,
    swaps: HashMap<SwapDigest, SwapId>,
    dial_queue: VecDeque<StateMachine>,
    /// Current number of concurrent outbound substreams being opened.
    dial_negotiated: u32,
    /// Maximum number of concurrent outbound substreams being opened. Value is
    /// never modified.
    max_dial_negotiated: u32,
}

pub enum OutgoingEvent {
    FetchSwapId(SwapDigest),
}

pub enum IncomingEvent {
    SwapIdFound(SwapId),
    AnnounceSwap(SwapDigest),
}

impl AnnounceHandler {
    pub fn new() -> Self {
        AnnounceHandler {
            swaps: HashMap::new(),
            events_out: VecDeque::new(),
            dial_queue: VecDeque::new(),
            dial_negotiated: 0,
            max_dial_negotiated: 4,
        }
    }
}

/// A new protocols handler is instantiated for each remote physical connection.
/// Multiple instances of the same "protocol" can be handled by the same
/// handler.
impl ProtocolsHandler for AnnounceHandler {
    /// consumed by the handler
    type InEvent = StateMachine;
    /// produced by the handler
    type OutEvent = StateMachine;
    type Error = AnnounceFailure;
    type InboundProtocol = StateMachine;
    type OutboundProtocol = StateMachine;
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<StateMachine> {
        SubstreamProtocol::new(StateMachine)
    }

    /// Injects/registers? succesfully upgraded inbound connection?
    fn inject_fully_negotiated_inbound(
        &mut self,
        output: <Self::InboundProtocol as InboundUpgradeSend>::Output,
    ) {
        // An announcement remote peer has been confirmed.
        self.events_out.push_front(output.into());
    }

    /// Injects/registers? succesfully upgraded outbound connection?
    fn inject_fully_negotiated_outbound(
        &mut self,
        output: <Self::InboundProtocol as InboundUpgradeSend>::Output,
        _info: (),
    ) {
        self.dial_negotiated -= 1;

        // if self.dial_negotiated == 0 && self.dial_queue.is_empty() {
        //     self.keep_alive = KeepAlive::Until(Instant::now() +
        // self.inactive_timeout); }

        // An announcement initiated by the local peer was confirmed by the remote.
        self.events_out.push_front(output);
    }

    /// inject InitiateAnnounce(SwapDigest) event from outside (network
    /// behaviour?),
    fn inject_event(&mut self, event: Self::InEvent) {
        self.dial_queue.push(&event);
    }

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

    /// kill connection if confirmation received ie. announce protocol has
    /// completed
    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Yes
    }

    fn poll(
        &mut self,
        cx: &mut Context,
    ) -> Poll<
        ProtocolsHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
            Self::Error,
        >,
    > {
        // propogate messages to network behaviour
        if !self.events_out.is_empty() {
            return Poll::Ready(ProtocolsHandlerEvent::Custom(
                self.events_out.remove(0).unwrap(),
            ));
        } else {
            self.events_out.shrink_to_fit();
        }

        // Initiate announce protocol
        if !self.dial_queue.is_empty() {
            if self.dial_negotiated < self.max_dial_negotiated {
                self.dial_negotiated += 1;
                return Poll::Ready(ProtocolsHandlerEvent::OutboundSubstreamRequest {
                    protocol: SubstreamProtocol::new(self.dial_queue.remove(0).unwrap()),
                    info: (),
                });
            }
        } else {
            self.dial_queue.shrink_to_fit();
        }

        Poll::Pending
    }
}
