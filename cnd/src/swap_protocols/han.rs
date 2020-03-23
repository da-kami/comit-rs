mod create_swap;
mod events;
mod ledger_state;

pub use self::{create_swap::*, events::*, ledger_state::*};

use crate::{swap_protocols::SecretHash, timestamp::Timestamp};

#[derive(Clone, Copy, Debug)]
pub struct HtlcParams<L, A, I> {
    pub asset: A,
    pub ledger: L,
    pub redeem_identity: I,
    pub refund_identity: I,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}
