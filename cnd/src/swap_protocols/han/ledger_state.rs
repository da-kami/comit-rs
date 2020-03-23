use crate::swap_protocols::{
    han::events::{Funded, Redeemed, Refunded},
    Secret,
};
use serde::Serialize;
use strum_macros::EnumDiscriminants;

#[derive(Clone, Debug, PartialEq, EnumDiscriminants)]
#[strum_discriminants(
    name(HtlcState),
    derive(Serialize, Display),
    serde(rename_all = "SCREAMING_SNAKE_CASE")
)]
pub enum LedgerState<A, H, T> {
    NotDeployed,
    Funded {
        asset: A,
        fund_transaction: T,
        htlc_location: H,
    },
    Redeemed {
        asset: A,
        fund_transaction: T,
        htlc_location: H,
        redeem_transaction: T,
        secret: Secret,
    },
    Refunded {
        asset: A,
        fund_transaction: T,
        htlc_location: H,
        refund_transaction: T,
    },
}

impl<A, H, T> Default for LedgerState<A, H, T> {
    fn default() -> Self {
        LedgerState::NotDeployed
    }
}

impl<A, H, T> LedgerState<A, H, T> {
    pub fn transition_to_funded(&mut self, funded: Funded<A, H, T>) {
        let Funded { transaction } = funded;

        match std::mem::replace(self, LedgerState::default()) {
            LedgerState::NotDeployed => {
                *self = LedgerState::Funded {
                    fund_transaction: transaction,
                }
            }
            other => panic!("expected state NotDeployed, got {}", HtlcState::from(other)),
        }
    }

    pub fn transition_to_redeemed(&mut self, redeemed: Redeemed<T>) {
        let Redeemed {
            transaction,
            secret,
        } = redeemed;

        match std::mem::replace(self, LedgerState::default()) {
            LedgerState::Funded {
                htlc_location,
                asset,
                fund_transaction,
            } => {
                *self = LedgerState::Redeemed {
                    htlc_location,
                    fund_transaction,
                    redeem_transaction: transaction,
                    asset,
                    secret,
                }
            }
            other => panic!("expected state Funded, got {}", HtlcState::from(other)),
        }
    }

    pub fn transition_to_refunded(&mut self, refunded: Refunded<T>) {
        let Refunded { transaction } = refunded;

        match std::mem::replace(self, LedgerState::default()) {
            LedgerState::Funded {
                htlc_location,
                asset,
                fund_transaction,
            }
            | LedgerState::IncorrectlyFunded {
                htlc_location,
                asset,
                fund_transaction,
            } => {
                *self = LedgerState::Refunded {
                    htlc_location,
                    fund_transaction,
                    refund_transaction: transaction,
                    asset,
                }
            }
            other => panic!(
                "expected state Funded or IncorrectlyFunded, got {}",
                HtlcState::from(other)
            ),
        }
    }
}

impl Default for HtlcState {
    fn default() -> Self {
        HtlcState::NotDeployed
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for HtlcState {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        match g.next_u32() % 6 {
            0 => HtlcState::NotDeployed,
            1 => HtlcState::Funded,
            2 => HtlcState::Redeemed,
            3 => HtlcState::Refunded,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn not_deployed_serializes_correctly_to_json() {
        let state = HtlcState::NotDeployed;
        let serialized = serde_json::to_string(&state).unwrap();
        assert_eq!(serialized, r#""NOT_DEPLOYED""#);
    }
}
