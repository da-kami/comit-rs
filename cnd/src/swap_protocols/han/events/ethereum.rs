pub mod htlc_events;

use crate::{
    asset,
    asset::{ethereum::FromWei, Ether},
    btsieve::ethereum::{
        watch_for_contract_creation, watch_for_event, Cache, Event, Topic, Web3Connector,
    },
    ethereum::{Bytes, H256, U256},
    htlc_location, identity,
    swap_protocols::{
        han::{
            Funded, HtlcFunded, HtlcParams, HtlcRedeemed, HtlcRefunded, LedgerState, Redeemed,
            Refunded,
        },
        ledger::Ethereum,
        HtlcParams, Secret,
    },
    transaction,
};
use blockchain_contracts::ethereum::rfc003::{erc20_htlc::Erc20Htlc, ether_htlc::EtherHtlc};

use chrono::NaiveDateTime;
use std::cmp::Ordering;
use tracing_futures::Instrument;

lazy_static::lazy_static! {
    // TODO: Remove reference to 'rfc003' when blockchain contracts does
    // https://github.com/comit-network/blockchain-contracts/issues/47
    static ref REDEEM_LOG_MSG: H256 = blockchain_contracts::ethereum::rfc003::REDEEMED_LOG_MSG.parse().expect("to be valid hex");
    static ref REFUND_LOG_MSG: H256 = blockchain_contracts::ethereum::rfc003::REFUNDED_LOG_MSG.parse().expect("to be valid hex");
    /// keccak('Transfer(address,address,uint256)')
    static ref TRANSFER_LOG_MSG: H256 = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".parse().expect("to be valid hex");
}

#[async_trait::async_trait]
impl
    HtlcFunded<
        Ethereum,
        asset::Ether,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams<Ethereum, asset::Ether, identity::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>> {
        let (transaction, location) =
            watch_for_contract_creation(self, start_of_swap, htlc_params.bytecode())
                .instrument(tracing::info_span!("htlc_funded"))
                .await?;

        let asset = Ether::from_wei(transaction.transaction.value);

        Ok(Funded {
            asset,
            location,
            transaction,
        })
    }
}

#[async_trait::async_trait]
impl
    HtlcRedeemed<
        Ethereum,
        asset::Ether,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_redeemed(
        &self,
        _htlc_params: &HtlcParams<Ethereum, asset::Ether, identity::Ethereum>,
        funding: &Funded<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<transaction::Ethereum>> {
        let event = Event {
            address: funding.htlc_location,
            topics: vec![Some(Topic(*REDEEM_LOG_MSG))],
        };

        let (transaction, log) = watch_for_event(self, start_of_swap, event)
            .instrument(tracing::info_span!("htlc_redeemed"))
            .await?;

        let log_data = log.data.0.as_ref();
        let secret =
            Secret::from_vec(log_data).expect("Must be able to construct secret from log data");

        Ok(Redeemed {
            transaction,
            secret,
        })
    }
}

#[async_trait::async_trait]
impl
    HtlcRefunded<
        Ethereum,
        asset::Ether,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_refunded(
        &self,
        _htlc_params: &HtlcParams<Ethereum, asset::Ether, identity::Ethereum>,
        funding: &Funded<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<transaction::Ethereum>> {
        let event = Event {
            address: funding.htlc_location,
            topics: vec![Some(Topic(*REFUND_LOG_MSG))],
        };

        let (transaction, _) = watch_for_event(self, start_of_swap, event)
            .instrument(tracing::info_span!("htlc_refunded"))
            .await?;

        Ok(Refunded { transaction })
    }
}

impl From<HtlcParams<Ethereum, asset::Ether, identity::Ethereum>> for EtherHtlc {
    fn from(htlc_params: HtlcParams<Ethereum, asset::Ether, identity::Ethereum>) -> Self {
        let refund_address = blockchain_contracts::ethereum::Address(htlc_params.refund_identity.0);
        let redeem_address = blockchain_contracts::ethereum::Address(htlc_params.redeem_identity.0);

        EtherHtlc::new(
            htlc_params.expiry.into(),
            refund_address,
            redeem_address,
            htlc_params.secret_hash.into(),
        )
    }
}

impl HtlcParams<Ethereum, asset::Ether, identity::Ethereum> {
    pub fn bytecode(&self) -> Bytes {
        EtherHtlc::from(self.clone()).into()
    }
}
