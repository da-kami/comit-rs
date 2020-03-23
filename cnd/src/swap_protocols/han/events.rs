use crate::swap_protocols::{han::HtlcParams, Secret};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Funded<A, H, T> {
    pub asset: A,
    pub location: H,
    pub transaction: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Redeemed<T> {
    pub transaction: T,
    pub secret: Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Refunded<T> {
    pub transaction: T,
}

#[async_trait::async_trait]
pub trait HtlcFunded<L, A, H, I, T>: Send + Sync + Sized + 'static {
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams<L, A, I>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<A, H, T>>;
}

#[async_trait::async_trait]
pub trait HtlcRedeemed<L, A, H, I, T>: Send + Sync + Sized + 'static {
    async fn htlc_redeemed(
        &self,
        htlc_params: &HtlcParams<L, A, I>,
        funding: &Funded<A, H, T>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<T>>;
}

#[async_trait::async_trait]
pub trait HtlcRefunded<L, A, H, I, T>: Send + Sync + Sized + 'static {
    async fn htlc_refunded(
        &self,
        htlc_params: &HtlcParams<L, A, I>,
        funding: &Funded<A, H, T>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<T>>;
}
