use crate::{db::AcceptedSwap, swap_protocols::han};

/// Glue the rfc003 protocol design (currently maintained in the HTTP API and
/// database layers) together with the new split protocol design.

/// Splits an rfc003 accepted swap into separate protocols for Han-Han swaps.
/// Returns HTLC parameters (alpha, beta).
pub fn split_swap_rfc003_into_han_han<AL, BL, AA, BA, AI, BI>(
    accepted: AcceptedSwap<AL, BL, AA, BA, AI, BI>,
) -> (han::HtlcParams<AL, AA, AI>, han::HtlcParams<BL, BA, BI>) {
    unimplemented!()
}
