use crate::{
    network::protocols::announce::{
        handler::{self, Handler, HandlerEvent},
        protocol::{OutboundConfig, ReplySubstream},
        SwapDigest,
    },
    swap_protocols::SwapId,
};
use libp2p::{
    core::{ConnectedPoint, Multiaddr, PeerId},
    swarm::{
        NegotiatedSubstream, NetworkBehaviour, NetworkBehaviourAction,
        NetworkBehaviourEventProcess, PollParameters, ProtocolsHandler,
    },
};
use std::{
    collections::{HashMap, VecDeque},
    task::{Context, Poll},
};

/// Network behaviour that announces a swap to peer by sending a `swap_digest`
/// and receives the `swap_id` back.
#[derive(Debug)]
pub struct Announce {
    /// Pending events to be emitted when polled.
    events: VecDeque<NetworkBehaviourAction<OutboundConfig, BehaviourEvent>>,
    address_book: HashMap<PeerId, Multiaddr>,
}

impl Announce {
    /// This is how data flows into the network behaviour from the application
    /// when acting in the Role of Alice.
    pub fn start_announce_protocol(&mut self, outbound_config: OutboundConfig, peer_id: &PeerId) {
        self.events.push_back(NetworkBehaviourAction::SendEvent {
            peer_id: peer_id.clone(),
            event: outbound_config,
        });
    }

    pub fn add_peer(&mut self, peer_id: PeerId, addr: Multiaddr) {
        self.address_book.insert(peer_id, addr);
    }
}

impl Default for Announce {
    fn default() -> Self {
        Announce {
            events: VecDeque::new(),
            address_book: HashMap::new(),
        }
    }
}

impl NetworkBehaviour for Announce {
    type ProtocolsHandler = Handler;
    type OutEvent = BehaviourEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Handler::default()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        if let Some(addr) = self.address_book.get(peer_id) {
            tracing::debug!("fetched peer: {} addr: {}", peer_id, addr.clone());
            vec![addr.clone()]
        } else {
            vec![]
        }
    }

    fn inject_connected(&mut self, peer_id: PeerId, _endpoint: ConnectedPoint) {}

    fn inject_disconnected(&mut self, _peer_id: &PeerId, _: ConnectedPoint) {}

    fn inject_node_event(&mut self, peer_id: PeerId, event: HandlerEvent) {
        match event {
            HandlerEvent::ReceivedConfirmation(confirmed) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourEvent::ReceivedConfirmation {
                        peer: peer_id,
                        swap_id: confirmed.swap_id,
                        swap_digest: confirmed.swap_digest,
                    },
                ));
            }
            HandlerEvent::AwaitingConfirmation(sender) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourEvent::AwaitingConfirmation {
                        peer: peer_id,
                        io: sender,
                    },
                ));
            }
            HandlerEvent::Error(error) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourEvent::Error {
                        peer: peer_id,
                        error,
                    },
                ));
            }
        }
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<
        NetworkBehaviourAction<
            <Self::ProtocolsHandler as ProtocolsHandler>::InEvent,
            Self::OutEvent,
        >,
    > {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}

/// Event emitted  by the `Announce` behaviour.
#[derive(Debug)]
pub enum BehaviourEvent {
    /// This event created when a confirmation message containing a `swap_id` is
    /// received in response to an announce message containing a
    /// `swap_digest`. The Event contains both the swap id and
    /// the swap digest. The announce message is sent by Alice to Bob.
    ReceivedConfirmation {
        /// The peer (Bob) that the swap has been announced to.
        peer: PeerId,
        /// The swap_id returned by the peer (Bob).
        swap_id: SwapId,
        /// The swap_digest
        swap_digest: SwapDigest,
    },

    /// The event is created when a remote sends a `swap_digest`. The event
    /// contains a reply substream for the receiver to send back the
    /// `swap_id` that corresponds to the swap digest. Bob sends the
    /// confirmations message to Alice using the the reply substream.
    AwaitingConfirmation {
        /// The peer (Alice) that the reply substream is connected to.
        peer: PeerId,
        /// The substream (inc. `swap_digest`) to reply on (i.e., send
        /// `swap_id`).
        io: ReplySubstream<NegotiatedSubstream>,
    },

    /// Error while attempting to announce swap to the remote.
    Error {
        /// The peer with whom the error originated.
        peer: PeerId,
        /// The error that occurred.
        error: handler::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::{Announce, BehaviourEvent};
    use crate::network::protocols::announce::{protocol::OutboundConfig, SwapDigest};
    use async_std;
    use futures::{pin_mut, prelude::*};
    use libp2p::{
        core::{muxing::StreamMuxer, upgrade},
        identity,
        mplex::MplexConfig,
        multihash::{Hash, Multihash},
        secio::SecioConfig,
        swarm::{Swarm, SwarmEvent},
        tcp::TcpConfig,
        PeerId, Transport,
    };

    use std::{fmt, io};

    fn transport() -> (
        PeerId,
        impl Transport<
                Output = (
                    PeerId,
                    impl StreamMuxer<
                        Substream = impl Send,
                        OutboundSubstream = impl Send,
                        Error = impl Into<io::Error>,
                    >,
                ),
                Listener = impl Send,
                ListenerUpgrade = impl Send,
                Dial = impl Send,
                Error = impl fmt::Debug,
            > + Clone,
    ) {
        let id_keys = identity::Keypair::generate_ed25519();
        let peer_id = id_keys.public().into_peer_id();
        let transport = TcpConfig::new()
            .nodelay(true)
            .upgrade(upgrade::Version::V1)
            .authenticate(SecioConfig::new(id_keys))
            .multiplex(MplexConfig::new());
        (peer_id, transport)
    }

    fn random_swap_digest() -> SwapDigest {
        SwapDigest {
            inner: Multihash::random(Hash::Keccak256),
        }
    }

    #[test]
    fn send_announce_receive_confirmation() {
        let (mut alice_swarm, alice_peer_id) = {
            let (peer_id, transport) = transport();
            let protocol = Announce::default();
            let swarm = Swarm::new(transport, protocol, peer_id.clone());
            (swarm, peer_id)
        };

        let (mut bob_swarm, bob_peer_id) = {
            let (peer_id, transport) = transport();
            let protocol = Announce::default();
            let swarm = Swarm::new(transport, protocol, peer_id.clone());
            (swarm, peer_id)
        };

        Swarm::listen_on(&mut bob_swarm, "/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();

        let bob_addr: libp2p::core::Multiaddr = async_std::task::block_on(async {
            loop {
                let bob_swarm_fut = bob_swarm.next_event();
                pin_mut!(bob_swarm_fut);
                match bob_swarm_fut.await {
                    SwarmEvent::NewListenAddr(addr) => return addr,
                    _ => {}
                }
            }
        });

        alice_swarm.add_peer(bob_peer_id.clone(), bob_addr.clone());
        Swarm::dial(&mut alice_swarm, bob_peer_id.clone());

        let send_swap_digest = random_swap_digest();
        let outbound_config = OutboundConfig::new(send_swap_digest.clone());

        alice_swarm.start_announce_protocol(outbound_config, &bob_peer_id);

        async_std::task::block_on(async move {
            loop {
                let bob_swarm_fut = bob_swarm.next_event();
                pin_mut!(bob_swarm_fut);
                match bob_swarm_fut.await {
                    SwarmEvent::Behaviour(behavior_event) => {
                        // never enters this block causing the test to hang
                        if let BehaviourEvent::AwaitingConfirmation { peer, io } = behavior_event {
                            assert_eq!(io.swap_digest, send_swap_digest);
                            // assert_eq!(peer, peer)
                            return;
                        }
                    }
                    SwarmEvent::Connected(peer_id) => {
                        assert_eq!(alice_peer_id.clone(), peer_id);
                    }
                    _ => {}
                }
            }
        })
    }
}
