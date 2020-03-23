use crate::swap_protocols::{
    han::{
        Funded, HtlcFunded, HtlcParams, HtlcRedeemed, HtlcRefunded, LedgerState, Redeemed, Refunded,
    },
    state, InsertFailedSwap, SwapId,
};
use chrono::NaiveDateTime;
use futures::future::{self, Either};
use genawaiter::{
    sync::{Co, Gen},
    GeneratorState,
};
use std::sync::Arc;

/// Returns a future that tracks a Han swap on a single ledger.
///
/// The current implementation is naive in the sense that it does not take into
/// account situations where it is clear that no more events will happen even
/// though in theory, there could. For example:
///
/// - funded
/// - refunded
///
/// It is highly unlikely for Bob to fund the HTLC now, yet the current
/// implementation is still waiting for that.
pub async fn create_watcher<D, S, L, A, H, I, T>(
    dependencies: D,
    ledger_state: Arc<S>,
    id: SwapId,
    htlc_params: HtlcParams<L, A, I>,
    accepted_at: NaiveDateTime,
) where
    D: InsertFailedSwap
        + HtlcFunded<L, A, H, I, T>
        + HtlcRedeemed<L, A, H, I, T>
        + HtlcRefunded<L, A, H, I, T>,
    S: state::Update<LedgerState<A, H, T>, SwapEvent<A, H, T>>
        + state::Insert<LedgerState<A, H, T>>,
    L: Clone,
    A: Ord + Clone,
    H: Clone,
    I: Clone,
    T: Clone,
{
    ledger_state
        .insert(id, LedgerState::<A, H, T>::NotDeployed)
        .await;

    // construct a generator that watches alpha and beta ledger concurrently
    let mut generator = Gen::new({
        |co| async {
            watch_ledger::<D, L, A, H, I, T>(&dependencies, co, htlc_params, accepted_at).await
        }
    });

    loop {
        // wait for events to be emitted as the generator executes
        match generator.async_resume().await {
            // every event that is yielded is passed on
            GeneratorState::Yielded(event) => {
                tracing::info!("swap {} yielded event {}", id, event);
                ledger_state.update(&id, event).await;
            }
            // the generator stopped executing, this means there are no more events that can be
            // watched.
            GeneratorState::Complete(Ok(_)) => {
                tracing::info!("swap {} finished", id);
                return;
            }
            GeneratorState::Complete(Err(e)) => {
                tracing::error!("swap {} failed with {:?}", id, e);
                dependencies.insert_failed_swap(&id);
                return;
            }
        }
    }
}

/// Returns a future that waits for events to happen on the ledger.
///
/// Each event is yielded through the controller handle (co) of the coroutine.
async fn watch_ledger<D, L, A, H, I, T>(
    dependencies: &D,
    co: Co<SwapEvent<A, H, T>>,
    htlc_params: HtlcParams<L, A, I>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    D: HtlcFunded<L, A, H, I, T> + HtlcRedeemed<L, A, H, I, T> + HtlcRefunded<L, A, H, I, T>,
    Redeemed<T>: Clone,
    Refunded<T>: Clone,
{
    let funded = dependencies
        .htlc_funded(&htlc_params, start_of_swap)
        .await?;
    co.yield_(SwapEvent::Funded(funded)).await;

    let redeemed = dependencies.htlc_redeemed(&htlc_params, &funded, start_of_swap);
    let refunded = dependencies.htlc_refunded(&htlc_params, &funded, start_of_swap);

    match future::try_select(redeemed, refunded).await {
        Ok(Either::Left((redeemed, _))) => {
            co.yield_(SwapEvent::Redeemed(redeemed.clone())).await;
        }
        Ok(Either::Right((refunded, _))) => {
            co.yield_(SwapEvent::Refunded(refunded.clone())).await;
        }
        Err(either) => {
            let (error, _other_future) = either.factor_first();

            return Err(error);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, strum_macros::Display)]
pub enum SwapEvent<A, H, T> {
    Funded(Funded<A, H, T>),
    Redeemed(Redeemed<T>),
    Refunded(Refunded<T>),
}
