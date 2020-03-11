use crate::swap_protocols::swap_id::SwapId;

pub trait Insert<S>: Send + Sync + 'static {
    fn insert(&self, key: SwapId, value: S);
}

pub trait Get<S>: Send + Sync + 'static {
    fn get(&self, key: &SwapId) -> anyhow::Result<Option<S>>;
}

pub trait Update<S, E>: Send + Sync + 'static {
    fn update(&self, key: &SwapId, update: E);
}
