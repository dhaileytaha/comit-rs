#![allow(clippy::type_repetition_in_bounds)]
use crate::{
    db::{AcceptedSwap, DetermineTypes, LoadAcceptedSwap, Retrieve, Sqlite},
    seed::Seed,
    swap_protocols::{
        ledger::LedgerConnectors,
        rfc003::{
            state_store::{InMemoryStateStore, StateStore},
            Spawn,
        },
    },
};
use futures::Stream;
use std::sync::Arc;

#[allow(clippy::cognitive_complexity)]
pub async fn load_swaps_from_database(
    ledger_events: LedgerConnectors,
    state_store: Arc<InMemoryStateStore>,
    seed: Seed,
    db: Sqlite,
) -> anyhow::Result<()> {
    log::debug!("loading swaps from database ...");

    for swap in db.all().await?.iter() {
        let swap_id = swap.swap_id;
        log::debug!("got swap from database: {}", swap_id);

        let types = db.determine_types(&swap_id).await?;

        with_swap_types!(types, {
            let accepted: Result<AcceptedSwap<AL, BL, AA, BA>, anyhow::Error> =
                db.load_accepted_swap(swap_id.clone()).await;

            match accepted {
                Ok((request, accept)) => {
                    match types.role {
                        Role::Alice => {
                            let state = alice::State::accepted(request.clone(), accept, seed);
                            state_store.insert(swap_id, state);

                            let receiver = ledger_events.spawn(request, accept);

                            tokio::spawn(receiver.for_each({
                                let state_store = state_store.clone();
                                move |update| {
                                    state_store
                                        .update::<alice::State<AL, BL, AA, BA>>(&swap_id, update);
                                    Ok(())
                                }
                            }));
                        }
                        Role::Bob => {
                            let state = bob::State::accepted(request.clone(), accept, seed);
                            state_store.insert(swap_id, state);

                            let receiver = ledger_events.spawn(request, accept);

                            tokio::spawn(receiver.for_each({
                                let state_store = state_store.clone();
                                move |update| {
                                    state_store
                                        .update::<bob::State<AL, BL, AA, BA>>(&swap_id, update);
                                    Ok(())
                                }
                            }));
                        }
                    };
                }
                Err(e) => log::error!("failed to load swap: {}, continuing ...", e),
            };
        });
    }
    Ok(())
}