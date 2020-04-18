#[cfg(test)]
mod tests {
    use crate::db::{
        new_db::{han_han_swap::Retrieve, new_save::Save, HanBitcoin, HanEthereum, NewSwap},
        Sqlite,
    };
    use std::path::Path;

    // TODO return anyhow and remove all unwraps.
    // need to fix `can only return types that implement
    // `std::process::Termination`` and don't know how :D
    #[test]
    fn test_store_load_swap_details() {
        let swap = NewSwap {
            local_swap_id: Default::default(),
        };

        let han_bitcoin = HanBitcoin {
            local_swap_id: swap.local_swap_id,
        };
        let han_ethereum = HanEthereum {
            local_swap_id: swap.local_swap_id,
        };

        let db = Sqlite::new(&Path::new(":memory:")).unwrap();

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                db.save(swap.clone(), han_bitcoin.clone(), han_ethereum.clone())
                    .await?;
                let loaded_swap = Retrieve::get(&db, &swap.local_swap_id).await?;
                anyhow::Result::<_>::Ok(loaded_swap)
            })
            .unwrap();

        assert_eq!(result, swap);
    }

    #[test]
    fn test_store_load_full_swap() {
        let db = Sqlite::new(&Path::new(":memory:")).unwrap();

        let other_swap = NewSwap {
            local_swap_id: Default::default(),
        };
        let han_bitcoin = HanBitcoin {
            local_swap_id: other_swap.local_swap_id,
        };
        let han_ethereum = HanEthereum {
            local_swap_id: other_swap.local_swap_id,
        };

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                db.save(
                    other_swap.clone(),
                    han_bitcoin.clone(),
                    han_ethereum.clone(),
                )
                .await?;
                anyhow::Result::<_>::Ok(())
            })
            .unwrap();

        let swap = NewSwap {
            local_swap_id: Default::default(),
        };
        let han_bitcoin = HanBitcoin {
            local_swap_id: swap.local_swap_id,
        };
        let han_ethereum = HanEthereum {
            local_swap_id: swap.local_swap_id,
        };

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                db.save(swap.clone(), han_bitcoin.clone(), han_ethereum.clone())
                    .await?;
                anyhow::Result::<_>::Ok(())
            })
            .unwrap();

        let (loaded_swap, loaded_han_bitcoin, loaded_han_ethereum) = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                let (loaded_swap, han_bitcoin, han_ethereum) =
                    Retrieve::get_han_han(&db, &swap.local_swap_id).await?;
                anyhow::Result::<_>::Ok((loaded_swap, han_bitcoin, han_ethereum))
            })
            .unwrap();

        assert_ne!(loaded_swap, other_swap);

        assert_eq!(loaded_swap, swap);
        assert_eq!(loaded_han_bitcoin, han_bitcoin);
        assert_eq!(loaded_han_ethereum, han_ethereum);
    }
}
