use crate::{SwapDigest, SwapId};
use futures::{future::BoxFuture, AsyncRead, AsyncWrite};
use libp2p::core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use serde::{de::DeserializeOwned, Serialize};
use std::{io, iter, marker::PhantomData};

const INFO: &'static str = "/comit/swap/announce/1.0.0";

/// The announce protocol works as follows:
/// - Dialer sends SwapDigest to listener
/// - Listener receives SwapDigest and sends SwapId
/// - Dialer reads SwapId
pub struct AnnounceAndReceiveConfirmation(SwapId);
pub struct ConfirmationReceived(SwapDigest, SwapId);

impl UpgradeInfo for Outbound {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(INFO.as_bytes())
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for AnnounceAndReceiveConfirmation
where
    TSocket: AsyncWrite + Unpin + Send + 'static,
{
    type Output = ConfirmationReceived;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    /// Writes the SwapDigest to the substream and reads the SwapId from peer.
    fn upgrade_outbound(self, mut socket: TSocket, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading outbound connection for {}",
            String::from_utf8_lossy(info)
        );
        Box::pin(async move {
            match self {
                AnnounceAndReceiveConfirmation::RequestForConfirmation(digest) => {
                    let bytes = serde_json::to_vec(&swap_id)?;
                    upgrade::write_one(&mut socket, &bytes).await?;
                    Ok()
                }
                AnnounceAndReceiveConfirmation::Confirmed(digest, swap_id) => {
                    let bytes = serde_json::to_vec(&swap_id)?;
                    upgrade::write_one(&mut socket, &bytes).await?;
                }
            }
        })
    }
}

pub struct ReceiveAnnounceAndSendConfirmation;
pub struct AnnounceReceived(SwapDigest);

impl UpgradeInfo for ReceiveAnnounceAndSendConfirmation {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(INFO.as_bytes())
    }
}

impl<TSocket> InboundUpgrade<TSocket> for ReceiveAnnounceAndSendConfirmation
where
    TSocket: AsyncRead + Unpin + Send + 'static,
{
    type Output = AnnounceReceived;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    /// Reads the SwapDigest from the substream and writes the SwapId to peer.
    fn upgrade_inbound(self, mut socket: TSocket, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading inbound connection for {}",
            String::from_utf8_lossy(info)
        );
        Box::pin(async move {
            let message = upgrade::read_one(&mut socket, 1024).await?;
            let mut de = serde_json::Deserializer::from_slice(&message);
            let swap_digest = SwapDigest::deserialize(&mut de)?.unwrap();

            Ok(AnnounceReceived(swap_digest))
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read a message from the socket")]
    Read(#[from] upgrade::ReadOneError),
    #[error("failed to write the message to the socket")]
    Write(#[from] io::Error),
    #[error("failed to serialize/deserialize the message")]
    Serde(#[from] serde_json::Error),
}

// TODO: Add unit test
