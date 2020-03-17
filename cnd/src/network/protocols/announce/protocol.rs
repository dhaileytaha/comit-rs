use crate::{network::protocols::announce::SwapDigest, swap_protocols::SwapId};
use futures::prelude::*;
use libp2p::core::upgrade::{self, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use serde::Deserialize;
use std::{io, iter, pin::Pin};

const INFO: &str = "/comit/swap/announce/1.0.0";

/// Configuration for an upgrade to the `Announce` protocol on the outbound
/// side.
#[derive(Debug, Clone)]
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

impl<C> OutboundUpgrade<C> for OutboundConfig
where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Output = Confirmed;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, mut socket: C, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading outbound connection for {}",
            String::from_utf8_lossy(info)
        );
        Box::pin(async move {
            let bytes = serde_json::to_vec(&self.swap_digest)?;
            upgrade::write_one(&mut socket, &bytes).await?;
            // FIXME: Is this correct (do we need a close of the write end for some reason
            // before reading)?
            socket.close().await?;

            let message = upgrade::read_one(&mut socket, 1024).await?;
            let mut de = serde_json::Deserializer::from_slice(&message);
            let swap_id = SwapId::deserialize(&mut de)?;
            tracing::trace!("Received: {}", swap_id);

            Ok(Confirmed {
                swap_digest: self.swap_digest.clone(),
                swap_id,
            })
        })
    }
}

#[derive(Debug)]
pub struct Confirmed {
    pub swap_digest: SwapDigest,
    pub swap_id: SwapId,
}

/// Configuration for an upgrade to the `Announce` protocol on the inbound side.
#[derive(Debug, Clone, Copy)]
pub struct InboundConfig {}

impl Default for InboundConfig {
    fn default() -> Self {
        InboundConfig {}
    }
}

impl UpgradeInfo for InboundConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(INFO.as_bytes())
    }
}

impl<C> InboundUpgrade<C> for InboundConfig
where
    C: AsyncRead + Unpin + Send + 'static,
{
    type Output = ReplySubstream<C>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, mut socket: C, info: Self::Info) -> Self::Future {
        tracing::trace!(
            "Upgrading inbound connection for {}",
            String::from_utf8_lossy(info)
        );

        Box::pin(async move {
            let message = upgrade::read_one(&mut socket, 1024).await?;
            let mut de = serde_json::Deserializer::from_slice(&message);
            let swap_digest = SwapDigest::deserialize(&mut de)?;
            Ok(ReplySubstream {
                io: socket,
                swap_digest,
            })
        })
    }
}

/// The substream on which a reply is expected to be sent.
#[derive(Debug)]
pub struct ReplySubstream<T> {
    pub io: T,
    pub swap_digest: SwapDigest,
}

impl<T> ReplySubstream<T>
where
    T: AsyncWrite + Unpin,
{
    /// Sends back the requested information on the substream i.e., the
    /// `swap_id`.
    ///
    /// Consumes the substream, returning a reply future that resolves
    /// when the reply has been sent on the underlying connection.
    pub async fn send(mut self, swap_id: SwapId) -> impl Future<Output = Result<(), Error>> {
        tracing::trace!("Sending: {}", swap_id);
        async move {
            let bytes = serde_json::to_vec(&swap_id)?;
            Ok(upgrade::write_one(&mut self.io, &bytes).await?)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::oneshot;
    use libp2p::{
        core::{
            upgrade::{self, apply_inbound, apply_outbound},
            Transport,
        },
        multihash::Multihash,
        tcp::TcpConfig,
    };
    use std::str::FromStr;

    fn swap_digest() -> SwapDigest {
        let v = Vec::from("abcdefg");
        SwapDigest {
            inner: Multihash::from_bytes(v).unwrap(),
        }
    }

    #[tokio::test]
    async fn correct_transfer() {
        let send_swap_digest = swap_digest();
        let send_swap_id = SwapId::from_str("ad2652ca-ecf2-4cc6-b35c-b4351ac28a34").unwrap();

        let (tx, rx) = oneshot::channel();

        tokio::task::spawn({
            let send_swap_digest = send_swap_digest.clone();
            async move {
                let transport = TcpConfig::new();

                let mut listener = transport
                    .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                    .unwrap();

                let addr = listener
                    .next()
                    .await
                    .expect("some event")
                    .expect("no error")
                    .into_new_address()
                    .expect("listen address");
                tx.send(addr).unwrap();

                let socket = listener
                    .next()
                    .await
                    .unwrap()
                    .unwrap()
                    .into_upgrade()
                    .unwrap()
                    .0
                    .await
                    .unwrap();
                let sender = apply_inbound(socket, InboundConfig::default())
                    .await
                    .unwrap();
                let receive_swap_digest = sender.swap_digest.clone();

                assert_eq!(send_swap_digest, receive_swap_digest);

                sender.send(send_swap_id).await
            }
        });

        let transport = TcpConfig::new();

        let socket = transport.dial(rx.await.unwrap()).unwrap().await.unwrap();
        let confirmed = apply_outbound(
            socket,
            OutboundConfig {
                swap_digest: send_swap_digest,
            },
            upgrade::Version::V1,
        )
        .await
        .unwrap();

        assert_eq!(send_swap_id, confirmed.swap_id)
    }
}
