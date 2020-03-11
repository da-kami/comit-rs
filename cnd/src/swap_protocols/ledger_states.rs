use crate::swap_protocols::{
    rfc003::{create_swap::SwapEvent, LedgerState},
    state::{Get, Insert, Update},
    swap_id::SwapId,
};
use std::{
    any::Any,
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Mutex,
};

#[derive(Default, Debug)]
pub struct AlphaLedgerState {
    states: Mutex<HashMap<SwapId, Box<dyn Any + Send>>>,
}

#[derive(Default, Debug)]
pub struct BetaLedgerState {
    states: Mutex<HashMap<SwapId, Box<dyn Any + Send>>>,
}

impl<S, L> Insert<S> for L
where
    L: DerefMut<Target = Mutex<HashMap<SwapId, Box<dyn Any + Send>>>> + Send + Sync + 'static,
    S: Send + 'static,
{
    fn insert(&self, key: SwapId, value: S) {
        let mut states = self.lock().unwrap();
        states.insert(key, Box::new(value));
    }
}

impl<S, L> Get<S> for L
where
    L: DerefMut<Target = Mutex<HashMap<SwapId, Box<dyn Any + Send>>>> + Send + Sync + 'static,
    S: Clone + Send + 'static,
{
    fn get(&self, key: &SwapId) -> anyhow::Result<Option<S>> {
        let states = self.lock().unwrap();
        match states.get(key) {
            Some(state) => match state.downcast_ref::<S>() {
                Some(state) => Ok(Some(state.clone())),
                None => Err(anyhow::anyhow!("invalid type")),
            },
            None => Ok(None),
        }
    }
}

impl<A, H, T, L> Update<LedgerState<A, H, T>, SwapEvent<A, H, T>> for L
where
    L: DerefMut<Target = Mutex<HashMap<SwapId, Box<dyn Any + Send>>>> + Send + Sync + 'static,
    LedgerState<A, H, T>: 'static,
{
    fn update(&self, key: &SwapId, event: SwapEvent<A, H, T>) {
        let mut states = self.lock().unwrap();
        let ledger_state = match states
            .get_mut(key)
            .and_then(|state| state.downcast_mut::<LedgerState<A, H, T>>())
        {
            Some(state) => state,
            None => {
                tracing::warn!("Value not found for key {}", key);
                return;
            }
        };

        match event {
            SwapEvent::Deployed(deployed) => ledger_state.transition_to_deployed(deployed),
            SwapEvent::Funded(funded) => ledger_state.transition_to_funded(funded),
            SwapEvent::Redeemed(redeemed) => {
                // what if redeemed.secret.hash() != secret_hash in request ??

                ledger_state.transition_to_redeemed(redeemed);
            }
            SwapEvent::Refunded(refunded) => ledger_state.transition_to_refunded(refunded),
        }
    }
}

impl Deref for AlphaLedgerState {
    type Target = Mutex<HashMap<SwapId, Box<dyn Any + Send>>>;
    fn deref(&self) -> &Self::Target {
        &self.states
    }
}

impl DerefMut for AlphaLedgerState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.states
    }
}

impl Deref for BetaLedgerState {
    type Target = Mutex<HashMap<SwapId, Box<dyn Any + Send>>>;

    fn deref(&self) -> &Self::Target {
        &self.states
    }
}

impl DerefMut for BetaLedgerState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.states
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{asset, htlc_location, transaction};
    use spectral::prelude::*;

    #[test]
    fn insert_and_get_ledger_state() {
        let ledger_states = AlphaLedgerState::default();
        let id = SwapId::default();

        ledger_states.insert(id, LedgerState::<asset::Bitcoin, htlc_location::Bitcoin, transaction::Bitcoin>::NotDeployed);

        let res: Option<LedgerState<asset::Bitcoin, htlc_location::Bitcoin, transaction::Bitcoin>> =
            ledger_states.get(&id).unwrap();
        assert_that(&res).contains_value(&LedgerState::NotDeployed);
    }
}
