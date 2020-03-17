use libp2p::PeerId;

use std::fmt;

mod behaviour;
mod handler;

pub use behaviour::Announce;

const PING_SIZE: usize = 32;

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

pub struct AnnonunceMessage;

pub struct Announcement;

pub struct Confirmation;

pub struct ConfirmationMessage;

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
