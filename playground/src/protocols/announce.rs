use futures::{future::BoxFuture, AsyncRead, AsyncWrite};
use libp2p::core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use serde::{de::DeserializeOwned, Serialize};
use std::{io, iter, marker::PhantomData};
use crate::{SwapId, SwapDigest};

const INFO: &'static str = "/comit/swap/announce/1.0.0";

/// The announce protocol works as follows:
/// - Dialer sends SwapDigest to listener
/// - Listener receives SwapDigest and sends SwapId
/// - Dialer reads SwapId

/// Represents a prototype for an upgrade to handle the sender side of the
/// announce protocol.
///
/// This struct contains the message that should be sent to the other peer.
#[derive(Clone, Copy, Debug)]
pub struct OutboundConfig {
    swap_digest: SwapDigest,
}

impl UpgradeInfo for OutboundConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(INFO.as_bytes())
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for OutboundConfig
    where
        TSocket: AsyncWrite + Unpin + Send + 'static,
{
    type Output = SwapId;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    /// Writes the SwapDigest to the substream and reads the SwapId from peer.
    fn upgrade_outbound(self, mut socket: TSocket, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading outbound connection for {}",
            String::from_utf8_lossy(info)
        );
        Box::pin(async move {
            let bytes = serde_json::to_vec(&self.swap_digest)?;
            upgrade::write_one(&mut socket, &bytes).await?;

            let message = upgrade::read_one(&mut socket, 1024).await?;
            let mut de = serde_json::Deserializer::from_slice(&message);
            let swap_id = SwapId::deserialize(&mut de)?;

            Ok(swap_id)
        })
    }
}

/// Represents a prototype for an upgrade to handle the receiver side of the
/// announce protocol.
#[derive(Clone, Copy, Debug)]
pub struct InboundConfig {
    swap_id: SwapId,
}

impl UpgradeInfo for InboundConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(INFO.as_bytes())
    }
}

impl<TSocket> InboundUpgrade<TSocket> for InboundConfig
    where
        TSocket: AsyncRead + Unpin + Send + 'static,
{
    type Output = SwapDigest;
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
            let swap_digest = SwapDigest::deserialize(&mut de)?;

            Ok(swap_digest)
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