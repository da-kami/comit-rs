use crate::{
    asset,
    db::AcceptedSwap,
    htlc_location, identity,
    swap_protocols::{
        self,
        han::{self, HtlcFunded, HtlcRedeemed, HtlcRefunded},
        ledger::{bitcoin, Ethereum},
        rfc003::{Accept, Request, SwapCommunication},
        state::Insert,
        Facade,
    },
    transaction,
};
use async_trait::async_trait;
use std::{default::Default, marker::PhantomData};
use tracing_futures::Instrument;

// TODO: Document Initiator.
pub struct Initiator<AL, BL, AA, BA, AI, BI> {
    phantom: PhantomData<(AL, BL, AA, BA, AI, BI)>,
}

impl<AL, BL, AA, BA, AI, BI> Default for Initiator<AL, BL, AA, BA, AI, BI> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

/// An accepted swap is an rfc003 construct.  We implement `init_accepted_swap`
/// for concrete types, making this function the boundary between the old
/// 'rfc003' protocol and new split protocols i.e., Han/HErc20.  (We are aiming
/// to maintain compatibility with rfc003, for the moment, in the HTTP API and
/// database layers.)

#[async_trait]
pub trait InitAcceptedSwap<AL, BL, AA, BA, AI, BI> {
    /// Initialize an rfc003 accepted swap for the Han-Han, HErc20-Han,
    /// Han-HErc20 protocols.
    async fn init_accepted_swap(
        &self,
        dependencies: &Facade,
        accepted: AcceptedSwap<AL, BL, AA, BA, AI, BI>,
    ) -> anyhow::Result<()>;
}

#[async_trait]
impl
    InitAcceptedSwap<
        bitcoin::Mainnet,
        Ethereum,
        asset::Bitcoin,
        asset::Ether,
        identity::Bitcoin,
        identity::Ethereum,
    >
    for Initiator<
        bitcoin::Mainnet,
        Ethereum,
        asset::Bitcoin,
        asset::Ether,
        identity::Bitcoin,
        identity::Ethereum,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            bitcoin::Mainnet,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<()> {
        let (request, accept, accepted_at) = accepted.clone();
        let id = request.swap_id;

        facade
            .insert(id, SwapCommunication::Accepted {
                request: request.clone(),
                response: accept,
            })
            .await;

        let (alpha, beta) = swap_protocols::split_swap_rfc003_into_han_han(accepted.clone());

        tracing::trace!("initialising accepted swap: {}", id);

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                bitcoin::Mainnet,
                asset::Bitcoin,
                htlc_location::Bitcoin,
                identity::Bitcoin,
                transaction::Bitcoin,
            >(
                facade.clone(),
                facade.alpha_ledger_state.clone(),
                id,
                alpha,
                accepted_at,
            )
            .instrument(tracing::info_span!("alpha")),
        );

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                Ethereum,
                asset::Ether,
                htlc_location::Ethereum,
                identity::Ethereum,
                transaction::Ethereum,
            >(
                facade.clone(),
                facade.beta_ledger_state.clone(),
                id,
                beta,
                accepted_at,
            )
            .instrument(tracing::info_span!("beta")),
        );

        Ok(())
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        bitcoin::Testnet,
        Ethereum,
        asset::Bitcoin,
        asset::Ether,
        identity::Bitcoin,
        identity::Ethereum,
    >
    for Initiator<
        bitcoin::Testnet,
        Ethereum,
        asset::Bitcoin,
        asset::Ether,
        identity::Bitcoin,
        identity::Ethereum,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            bitcoin::Testnet,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<()> {
        let (request, accept, accepted_at) = accepted.clone();
        let id = request.swap_id;

        facade
            .insert(id, SwapCommunication::Accepted {
                request: request.clone(),
                response: accept,
            })
            .await;

        let (alpha, beta) = swap_protocols::split_swap_rfc003_into_han_han(accepted.clone());

        tracing::trace!("initialising accepted swap: {}", id);

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                bitcoin::Testnet,
                asset::Bitcoin,
                htlc_location::Bitcoin,
                identity::Bitcoin,
                transaction::Bitcoin,
            >(
                facade.clone(),
                facade.alpha_ledger_state.clone(),
                id,
                alpha,
                accepted_at,
            )
            .instrument(tracing::info_span!("alpha")),
        );

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                Ethereum,
                asset::Ether,
                htlc_location::Ethereum,
                identity::Ethereum,
                transaction::Ethereum,
            >(
                facade.clone(),
                facade.beta_ledger_state.clone(),
                id,
                beta,
                accepted_at,
            )
            .instrument(tracing::info_span!("beta")),
        );

        Ok(())
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        bitcoin::Regtest,
        Ethereum,
        asset::Bitcoin,
        asset::Ether,
        identity::Bitcoin,
        identity::Ethereum,
    >
    for Initiator<
        bitcoin::Regtest,
        Ethereum,
        asset::Bitcoin,
        asset::Ether,
        identity::Bitcoin,
        identity::Ethereum,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            bitcoin::Regtest,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<()> {
        let (request, accept, accepted_at) = accepted.clone();
        let id = request.swap_id;

        facade
            .insert(id, SwapCommunication::Accepted {
                request: request.clone(),
                response: accept,
            })
            .await;

        let (alpha, beta) = swap_protocols::split_swap_rfc003_into_han_han(accepted.clone());

        tracing::trace!("initialising accepted swap: {}", id);

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                bitcoin::Regtest,
                asset::Bitcoin,
                htlc_location::Bitcoin,
                identity::Bitcoin,
                transaction::Bitcoin,
            >(
                facade.clone(),
                facade.alpha_ledger_state.clone(),
                id,
                alpha,
                accepted_at,
            )
            .instrument(tracing::info_span!("alpha")),
        );

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                Ethereum,
                asset::Ether,
                htlc_location::Ethereum,
                identity::Ethereum,
                transaction::Ethereum,
            >(
                facade.clone(),
                facade.beta_ledger_state.clone(),
                id,
                beta,
                accepted_at,
            )
            .instrument(tracing::info_span!("beta")),
        );

        Ok(())
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        Ethereum,
        bitcoin::Mainnet,
        asset::Ether,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
    for Initiator<
        Ethereum,
        bitcoin::Mainnet,
        asset::Ether,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            Ethereum,
            bitcoin::Mainnet,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<()> {
        let (request, accept, accepted_at) = accepted.clone();
        let id = request.swap_id;

        facade
            .insert(id, SwapCommunication::Accepted {
                request: request.clone(),
                response: accept,
            })
            .await;

        let (alpha, beta) = swap_protocols::split_swap_rfc003_into_han_han(accepted.clone());

        tracing::trace!("initialising accepted swap: {}", id);

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                Ethereum,
                asset::Ether,
                htlc_location::Ethereum,
                identity::Ethereum,
                transaction::Ethereum,
            >(
                facade.clone(),
                facade.alpha_ledger_state.clone(),
                id,
                alpha,
                accepted_at,
            )
            .instrument(tracing::info_span!("alpha")),
        );

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                bitcoin::Mainnet,
                asset::Bitcoin,
                htlc_location::Bitcoin,
                identity::Bitcoin,
                transaction::Bitcoin,
            >(
                facade.clone(),
                facade.beta_ledger_state.clone(),
                id,
                beta,
                accepted_at,
            )
            .instrument(tracing::info_span!("beta")),
        );

        Ok(())
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        Ethereum,
        bitcoin::Testnet,
        asset::Ether,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
    for Initiator<
        Ethereum,
        bitcoin::Testnet,
        asset::Ether,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            Ethereum,
            bitcoin::Testnet,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<()> {
        let (request, accept, accepted_at) = accepted.clone();
        let id = request.swap_id;

        facade
            .insert(id, SwapCommunication::Accepted {
                request: request.clone(),
                response: accept,
            })
            .await;

        let (alpha, beta) = swap_protocols::split_swap_rfc003_into_han_han(accepted.clone());

        tracing::trace!("initialising accepted swap: {}", id);

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                Ethereum,
                asset::Ether,
                htlc_location::Ethereum,
                identity::Ethereum,
                transaction::Ethereum,
            >(
                facade.clone(),
                facade.alpha_ledger_state.clone(),
                id,
                alpha,
                accepted_at,
            )
            .instrument(tracing::info_span!("alpha")),
        );

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                bitcoin::Testnet,
                asset::Bitcoin,
                htlc_location::Bitcoin,
                identity::Bitcoin,
                transaction::Bitcoin,
            >(
                facade.clone(),
                facade.beta_ledger_state.clone(),
                id,
                beta,
                accepted_at,
            )
            .instrument(tracing::info_span!("beta")),
        );

        Ok(())
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        Ethereum,
        bitcoin::Regtest,
        asset::Ether,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
    for Initiator<
        Ethereum,
        bitcoin::Regtest,
        asset::Ether,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            Ethereum,
            bitcoin::Regtest,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
        phantom: PhantomData<(Ethereum, bitcoin::Regtest, asset::Ether, asset::Bitcoin)>,
    ) -> anyhow::Result<()> {
        let (request, accept, accepted_at) = accepted.clone();
        let id = request.swap_id;

        facade
            .insert(id, SwapCommunication::Accepted {
                request: request.clone(),
                response: accept,
            })
            .await;

        let (alpha, beta) = swap_protocols::split_swap_rfc003_into_han_han(accepted.clone());

        tracing::trace!("initialising accepted swap: {}", id);

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                Ethereum,
                asset::Ether,
                htlc_location::Ethereum,
                identity::Ethereum,
                transaction::Ethereum,
            >(
                facade.clone(),
                facade.alpha_ledger_state.clone(),
                id,
                alpha,
                accepted_at,
            )
            .instrument(tracing::info_span!("alpha")),
        );

        tokio::task::spawn(
            han::create_watcher::<
                _,
                _,
                bitcoin::Regtest,
                asset::Bitcoin,
                htlc_location::Bitcoin,
                identity::Bitcoin,
                transaction::Bitcoin,
            >(
                facade.clone(),
                facade.beta_ledger_state.clone(),
                id,
                beta,
                accepted_at,
            )
            .instrument(tracing::info_span!("beta")),
        );

        Ok(())
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        bitcoin::Mainnet,
        Ethereum,
        asset::Bitcoin,
        asset::Erc20,
        identity::Bitcoin,
        identity::Ethereum,
    >
    for Initiator<
        bitcoin::Mainnet,
        Ethereum,
        asset::Bitcoin,
        asset::Erc20,
        identity::Bitcoin,
        identity::Ethereum,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            bitcoin::Mainnet,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        bitcoin::Testnet,
        Ethereum,
        asset::Bitcoin,
        asset::Erc20,
        identity::Bitcoin,
        identity::Ethereum,
    >
    for Initiator<
        bitcoin::Testnet,
        Ethereum,
        asset::Bitcoin,
        asset::Erc20,
        identity::Bitcoin,
        identity::Ethereum,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            bitcoin::Testnet,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        bitcoin::Regtest,
        Ethereum,
        asset::Bitcoin,
        asset::Erc20,
        identity::Bitcoin,
        identity::Ethereum,
    >
    for Initiator<
        bitcoin::Regtest,
        Ethereum,
        asset::Bitcoin,
        asset::Erc20,
        identity::Bitcoin,
        identity::Ethereum,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            bitcoin::Regtest,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        Ethereum,
        bitcoin::Mainnet,
        asset::Erc20,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
    for Initiator<
        Ethereum,
        bitcoin::Mainnet,
        asset::Erc20,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            Ethereum,
            bitcoin::Mainnet,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        Ethereum,
        bitcoin::Testnet,
        asset::Erc20,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
    for Initiator<
        Ethereum,
        bitcoin::Testnet,
        asset::Erc20,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            Ethereum,
            bitcoin::Testnet,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
}

#[async_trait]
impl
    InitAcceptedSwap<
        Ethereum,
        bitcoin::Regtest,
        asset::Erc20,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
    for Initiator<
        Ethereum,
        bitcoin::Regtest,
        asset::Erc20,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
{
    async fn init_accepted_swap(
        facade: &Facade,
        accepted: AcceptedSwap<
            Ethereum,
            bitcoin::Regtest,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
}
