use crate::swap_protocols::swap_id::SwapId;
use std::{collections::HashMap, sync::Mutex};

#[derive(Default, Debug)]
pub struct SwapErrorState(Mutex<HashMap<SwapId, bool>>);

impl SwapErrorState {
    pub fn get(&self, id: &SwapId) -> bool {
        *self.0.lock().unwrap().get(id).unwrap_or(&false)
    }

    pub fn insert(&self, id: &SwapId) {
        let _ = self.0.lock().unwrap().insert(*id, true);
    }
}

pub trait InsertSwapError {
    fn insert_swap_error(&self, id: &SwapId);
}
