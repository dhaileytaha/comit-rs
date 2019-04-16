#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

use comit_node::{
    btsieve::BtsieveHttpClient,
    comit_client::Client,
    comit_i_routes,
    http_api::route_factory,
    logging, network,
    seed::Seed,
    settings::ComitNodeSettings,
    swap_protocols::{
        self,
        metadata_store::MetadataStore,
        rfc003::state_store::{InMemoryStateStore, StateStore},
        InMemoryMetadataStore, SwapId,
    },
};
use directories;
use futures::{stream, Future, Stream};
use libp2p::identity::{self, ed25519};
use std::{
    env::var,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

fn main() -> Result<(), failure::Error> {
    let settings = load_settings()?;
    logging::set_up_logging(&settings);

    log::info!("Starting up with {:#?}", settings);

    let mut runtime = tokio::runtime::Runtime::new()?;

    let metadata_store = Arc::new(InMemoryMetadataStore::default());
    let state_store = Arc::new(InMemoryStateStore::default());
    let btsieve_client = create_btsieve_api_client(&settings);

    let bob_protocol_dependencies = swap_protocols::bob::ProtocolDependencies {
        ledger_events: btsieve_client.clone().into(),
        metadata_store: Arc::clone(&metadata_store),
        state_store: Arc::clone(&state_store),
        seed: settings.comit.secret_seed,
    };

    let local_key_pair = derive_key_pair(&settings.comit.secret_seed);

    let transport = libp2p::build_development_transport(local_key_pair.clone());
    let behaviour = network::Behaviour::new(bob_protocol_dependencies, runtime.executor())?;

    let mut swarm = libp2p::Swarm::new(transport, behaviour, local_key_pair.public().into());

    libp2p::Swarm::listen_on(
        &mut swarm,
        format!("/ip4/0.0.0.0/tcp/{}", settings.comit.comit_listen.port())
            .parse()
            .unwrap(),
    )?;

    let shared_swarm = Arc::new(Mutex::new(swarm));

    let alice_protocol_dependencies = swap_protocols::alice::ProtocolDependencies {
        ledger_events: btsieve_client.into(),
        metadata_store: Arc::clone(&metadata_store),
        state_store: Arc::clone(&state_store),
        seed: settings.comit.secret_seed,
        client: shared_swarm.clone(),
    };

    let future = stream::poll_fn(move || shared_swarm.lock().unwrap().poll())
        .for_each(|_| Ok(()))
        .map_err(|e| {
            log::error!("failed with {:?}", e);
        });

    runtime.spawn(future);

    spawn_warp_instance(
        &settings,
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        alice_protocol_dependencies,
        &mut runtime,
    );

    spawn_comit_i_instance(&settings, &mut runtime);

    // Block the current thread.
    ::std::thread::park();
    Ok(())
}

fn derive_key_pair(secret_seed: &Seed) -> identity::Keypair {
    let bytes = secret_seed.sha256_with_seed(&[b"NODE_ID"]);
    let key = ed25519::SecretKey::from_bytes(bytes).expect("we always pass 32 bytes");
    identity::Keypair::Ed25519(key.into())
}

fn load_settings() -> Result<ComitNodeSettings, config::ConfigError> {
    match directories::UserDirs::new() {
        None => Err(config::ConfigError::Message(
            "Unable to determine user's home directory".to_string(),
        )),
        Some(dirs) => {
            let default_config = std::path::Path::join(dirs.home_dir(), ".config/comit_node");
            let comit_config_path = var_or_default(
                "COMIT_NODE_CONFIG_PATH",
                default_config.to_string_lossy().to_string(),
            );
            let run_mode_config = var_or_default("RUN_MODE", "development".into());
            let default_config = format!("{}/{}", comit_config_path.trim(), "default");
            let run_mode_config = format!("{}/{}", comit_config_path.trim(), run_mode_config);
            let settings = ComitNodeSettings::create(default_config, run_mode_config)?;
            Ok(settings)
        }
    }
}

fn create_btsieve_api_client(settings: &ComitNodeSettings) -> BtsieveHttpClient {
    BtsieveHttpClient::new(
        &settings.btsieve.url,
        settings.btsieve.bitcoin.poll_interval_secs,
        settings.btsieve.bitcoin.network.as_str(),
        settings.btsieve.ethereum.poll_interval_secs,
        settings.btsieve.ethereum.network.as_str(),
    )
}

fn spawn_warp_instance<T: MetadataStore<SwapId>, S: StateStore, C: Client>(
    settings: &ComitNodeSettings,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    protocol_dependencies: swap_protocols::alice::ProtocolDependencies<T, S, C>,
    runtime: &mut tokio::runtime::Runtime,
) {
    let routes = route_factory::create(
        metadata_store,
        state_store,
        protocol_dependencies,
        auth_origin(&settings),
    );

    let listen_addr = SocketAddr::new(settings.http_api.address, settings.http_api.port);

    log::info!("Starting HTTP server on {:?}", listen_addr);

    let server = warp::serve(routes).bind(listen_addr);

    runtime.spawn(server);
}

fn spawn_comit_i_instance(settings: &ComitNodeSettings, runtime: &mut tokio::runtime::Runtime) {
    if let Some(comit_i_settings) = &settings.web_gui {
        let routes = comit_i_routes::create();

        let listen_addr = SocketAddr::new(comit_i_settings.address, comit_i_settings.port);

        log::info!("Starting comit-i HTTP server on {:?}", listen_addr);

        let server = warp::serve(routes).bind(listen_addr);

        runtime.spawn(server);
    }
}

fn auth_origin(settings: &ComitNodeSettings) -> String {
    match &settings.web_gui {
        Some(http_socket) => format!("http://localhost:{}", http_socket.port),
        None => "http://localhost:8080".to_string(),
    }
}

fn var_or_default(name: &str, default: String) -> String {
    match var(name) {
        Ok(value) => {
            log::info!("Set {}={}", name, value);
            value
        }
        Err(_) => {
            eprintln!(
                "{} is not set, falling back to default: '{}' ",
                name, default
            );
            default
        }
    }
}
