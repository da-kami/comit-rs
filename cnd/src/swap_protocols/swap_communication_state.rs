use crate::swap_protocols::{
    state::{Get, Insert},
    swap_id::SwapId,
};
use std::{any::Any, clone::Clone, collections::HashMap, sync::Mutex};

#[derive(Default, Debug)]
pub struct SwapCommunicationState {
    states: Mutex<HashMap<SwapId, Box<dyn Any + Send>>>,
}

impl<S> Insert<S> for SwapCommunicationState
where
    S: Send + 'static,
{
    fn insert(&self, key: SwapId, value: S) {
        let mut states = self.states.lock().unwrap();
        states.insert(key, Box::new(value));
    }
}

impl<S> Get<S> for SwapCommunicationState
where
    S: Clone + Send + 'static,
{
    fn get(&self, key: &SwapId) -> anyhow::Result<Option<S>> {
        let states = self.states.lock().unwrap();
        match states.get(key) {
            Some(state) => match state.downcast_ref::<S>() {
                Some(state) => Ok(Some(state.clone())),
                None => Err(anyhow::anyhow!("invalid type")),
            },
            None => Ok(None),
        }
    }
}
