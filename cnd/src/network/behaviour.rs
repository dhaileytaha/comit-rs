use crate::network::protocol::{InboundProtocolConfig, Message, OutboundProtocolConfig};
use libp2p::{
    core::{ConnectedPoint, Multiaddr, PeerId},
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, OneShotHandler, PollParameters, ProtocolsHandler,
    },
};
use std::{
    collections::VecDeque,
    task::{Context, Poll},
};
use tracing::trace;

/// Network behaviour that handles the secret hash protocol.
#[derive(Debug)]
pub struct Behaviour {
    /// Events that need to be yielded to the outside when polling.
    events: VecDeque<NetworkBehaviourAction<OutboundProtocolConfig, OutEvent>>,
}

impl Default for Behaviour {
    fn default() -> Self {
        Behaviour {
            events: VecDeque::new(),
        }
    }
}

/// Event generated by the NetworkBehaviour and that the swarm will report back.
#[derive(Clone, Copy, Debug)]
pub enum OutEvent {
    Received(Message), // OutEvent containing secret hash message.
    Sent,              // Empty/nil OutEvent i.e., `()`.
}

/// OutEvent when a peer sends us a message.
impl From<Message> for OutEvent {
    fn from(msg: Message) -> OutEvent {
        OutEvent::Received(msg)
    }
}

/// OutEvent that occurs when we send a message.
impl From<()> for OutEvent {
    fn from(_: ()) -> Self {
        OutEvent::Sent
    }
}

impl NetworkBehaviour for Behaviour {
    type ProtocolsHandler = OneShotHandler<InboundProtocolConfig, OutboundProtocolConfig, OutEvent>;
    type OutEvent = OutEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        Vec::new() // Announce protocol takes care of this.
    }

    fn inject_connected(&mut self, _: PeerId, _: ConnectedPoint) {
        // Do nothing, announce protocol is going to take care of connections.
    }

    fn inject_disconnected(&mut self, _: &PeerId, _: ConnectedPoint) {
        // Do nothing, announce protocol is going to take care of connections.
    }

    fn inject_node_event(&mut self, peer_id: PeerId, event: OutEvent) {
        match event {
            OutEvent::Received(message) => {
                trace!("Received message event from {}: {:?}", peer_id, message);

                // Add the message to be dispatched to the user.
                self.events
                    .push_back(NetworkBehaviourAction::GenerateEvent(OutEvent::Received(
                        message,
                    )));
            }
            OutEvent::Sent => trace!("Received 'sent' event from {}", peer_id),
        }
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
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
