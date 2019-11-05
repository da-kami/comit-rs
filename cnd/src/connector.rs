use crate::{
    network::Network,
    seed::Seed,
    swap_protocols::{
        metadata_store::{self, InMemoryMetadataStore, MetadataStore},
        rfc003::{
            alice::{InitiateRequest, SendRequest, SpawnAlice},
            bob::SpawnBob,
            state_machine::SwapStates,
            state_store::{self, InMemoryStateStore, StateStore},
            ActorState,
        },
        LedgerConnectors, Metadata, SwapId,
    },
};
use futures::sync::oneshot::Sender;
use libp2p::PeerId;
use libp2p_comit::frame::Response;
use std::sync::Arc;

/// Collect all the connector trait bounds together under one trait.
pub trait Connect:
    Clone + MetadataStore + StateStore + Network + InitiateRequest + SpawnAlice + SpawnBob
{
}

/// Connector is used to connect incoming messages from the HTTP API with logic
/// that triggers outgoing messages on the libp2p layer.
#[allow(missing_debug_implementations)]
pub struct Connector<S> {
    pub deps: Arc<Dependencies>,
    pub swarm: Arc<S>, // S is the libp2p Swarm within a mutex.
}

impl<S> Connect for Connector<S> where S: Network + SendRequest {}

impl<S> Clone for Connector<S> {
    fn clone(&self) -> Self {
        Self {
            deps: Arc::clone(&self.deps),
            swarm: Arc::clone(&self.swarm),
        }
    }
}

/// Dependencies that are needed by both the libp2p network layer and the HTTP
/// API layer.
#[allow(missing_debug_implementations)]
pub struct Dependencies {
    pub ledger_events: LedgerConnectors,
    pub metadata_store: Arc<InMemoryMetadataStore>,
    pub state_store: Arc<InMemoryStateStore>,
    pub seed: Seed,
}

impl Clone for Dependencies {
    fn clone(&self) -> Self {
        Self {
            ledger_events: self.ledger_events.clone(),
            metadata_store: Arc::clone(&self.metadata_store),
            state_store: Arc::clone(&self.state_store),
            seed: self.seed,
        }
    }
}

impl<S> MetadataStore for Connector<S>
where
    S: Send + Sync + 'static,
{
    fn get(&self, key: SwapId) -> Result<Option<Metadata>, metadata_store::Error> {
        self.deps.metadata_store.get(key)
    }

    fn insert(&self, metadata: Metadata) -> Result<(), metadata_store::Error> {
        self.deps.metadata_store.insert(metadata)
    }

    fn all(&self) -> Result<Vec<Metadata>, metadata_store::Error> {
        self.deps.metadata_store.all()
    }
}

impl<S> StateStore for Connector<S>
where
    S: Send + Sync + 'static,
{
    fn insert<A: ActorState>(&self, key: SwapId, value: A) {
        self.deps.state_store.insert(key, value)
    }

    fn get<A: ActorState>(&self, key: &SwapId) -> Result<Option<A>, state_store::Error> {
        self.deps.state_store.get(key)
    }

    fn update<A: ActorState>(&self, key: &SwapId, update: SwapStates<A::AL, A::BL, A::AA, A::BA>) {
        self.deps.state_store.update::<A>(key, update)
    }
}

impl<S: Network> Network for Connector<S>
where
    S: Send + Sync + 'static,
{
    fn comit_peers(
        &self,
    ) -> Box<dyn Iterator<Item = (PeerId, Vec<libp2p::Multiaddr>)> + Send + 'static> {
        self.swarm.comit_peers()
    }

    fn listen_addresses(&self) -> Vec<libp2p::Multiaddr> {
        self.swarm.listen_addresses()
    }

    fn pending_request_for(&self, swap: SwapId) -> Option<Sender<Response>> {
        self.swarm.pending_request_for(swap)
    }
}