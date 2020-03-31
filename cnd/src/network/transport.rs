use libp2p::{
    core::{
        either::EitherError,
        muxing::StreamMuxerBox,
        transport::{boxed::Boxed, timeout::TransportTimeoutError},
        upgrade::{SelectUpgrade, Version},
        UpgradeError,
    },
    identity, PeerId, Transport,
};
use libp2p_dns::{DnsConfig, DnsErr};
use libp2p_mplex::MplexConfig;
use libp2p_secio::{SecioConfig, SecioError};
use libp2p_tcp::TcpConfig;
use std::{io, time::Duration};

pub type ComitTransport = Boxed<
    (PeerId, StreamMuxerBox),
    TransportTimeoutError<
        EitherError<
            EitherError<DnsErr<io::Error>, UpgradeError<SecioError>>,
            UpgradeError<EitherError<io::Error, io::Error>>,
        >,
    >,
>;

/// Builds a libp2p transport with the following features:
/// - TcpConnection
/// - DNS name resolution
/// - authentication via secio
/// - multiplexing via yamux or mplex
pub fn build_comit_transport(keypair: identity::Keypair) -> anyhow::Result<ComitTransport> {
    let transport = TcpConfig::new().nodelay(true);
    let transport = DnsConfig::new(transport)?;

    let transport = transport
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(keypair))
        .multiplex(SelectUpgrade::new(
            libp2p_yamux::Config::default(),
            MplexConfig::new(),
        ))
        .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
        .timeout(Duration::from_secs(20))
        .boxed();

    Ok(transport)
}
