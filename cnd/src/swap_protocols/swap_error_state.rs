use crate::swap_protocols::swap_id::SwapId;
use std::{collections::HashMap, sync::Mutex};

#[derive(Default, Debug)]
pub struct SwapErrorState(Mutex<HashMap<SwapId, bool>>);

impl SwapErrorState {
    pub fn has_failed(&self, id: &SwapId) -> bool {
        *self.0.lock().unwrap().get(id).unwrap_or(&false)
    }
}

pub trait InsertFailedSwap {
    fn insert_failed_swap(&self, id: &SwapId);
}

impl InsertFailedSwap for SwapErrorState {
    fn insert_failed_swap(&self, id: &SwapId) {
        let _ = self.0.lock().unwrap().insert(*id, true);
    }
}
