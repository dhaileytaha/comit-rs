use crate::{
    db::{
        new_db::{HanBitcoin, HanEthereum, NewSwap},
        schema,
        wrapper_types::custom_sql_types::Text,
        Error, Sqlite,
    },
    diesel::{ExpressionMethods, OptionalExtension, QueryDsl},
    swap_protocols::SwapId,
};
use async_trait::async_trait;
use diesel::{self, prelude::*, RunQueryDsl};
use schema::{han_bitcoin, han_ethereum, swaps};

diesel::allow_tables_to_appear_in_same_query!(swaps, han_bitcoin, han_ethereum);

/// Retrieve swaps from database.
#[async_trait]
pub trait Retrieve: Send + Sync + 'static {
    async fn get(&self, key: &SwapId) -> anyhow::Result<NewSwap>;
    async fn get_han_han(&self, key: &SwapId)
        -> anyhow::Result<(NewSwap, HanBitcoin, HanEthereum)>;
}

#[async_trait]
impl Retrieve for Sqlite {
    async fn get(&self, key: &SwapId) -> anyhow::Result<NewSwap> {
        use self::schema::swaps::dsl::*;

        let swap: QueryableSwap = self
            .do_in_transaction(|connection| {
                let key = Text(key);

                swaps
                    .filter(local_swap_id.eq(key))
                    .first(&*connection)
                    .optional()
            })
            .await?
            .ok_or(Error::SwapNotFound)?;

        Ok(NewSwap::from(swap))
    }

    async fn get_han_han(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<(NewSwap, HanBitcoin, HanEthereum)> {
        use schema::{han_bitcoin, han_ethereum, swaps};
        // TODO somehow this should be possible: let (swap, alpha, beta) :
        // (QueryableSwap, QueryableHanBTC,...) = however, I get an error that
        // Queryable is not implemented for (QueryableSwap, QueryableHanBTC, ...) see here: https://docs.diesel.rs/diesel/prelude/trait.QueryDsl.html#method.inner_join
        let swap: QueryableSwap = self
            .do_in_transaction(|connection| {
                let key = Text(key);

                swaps::table
                    .inner_join(
                        han_bitcoin::table.on(han_bitcoin::local_swap_id.eq(swaps::local_swap_id)),
                    )
                    .inner_join(
                        han_ethereum::table
                            .on(han_ethereum::local_swap_id.eq(swaps::local_swap_id)),
                    )
                    .select((swaps::id, swaps::local_swap_id))
                    .filter(swaps::local_swap_id.eq(key))
                    .first(connection)
                    .optional()
            })
            .await?
            .ok_or(Error::SwapNotFound)?;

        Ok((
            NewSwap::from(swap.clone()),
            HanBitcoin::from(swap.clone()),
            HanEthereum::from(swap),
        ))
    }
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct QueryableSwap {
    pub id: i32,
    pub local_swap_id: Text<SwapId>,
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct QueryableHanBitcoin {
    pub id: i32,
    pub local_swap_id: Text<SwapId>,
}

impl From<QueryableSwap> for NewSwap {
    fn from(swap: QueryableSwap) -> NewSwap {
        NewSwap {
            local_swap_id: *swap.local_swap_id,
        }
    }
}

impl From<QueryableSwap> for HanEthereum {
    fn from(swap: QueryableSwap) -> HanEthereum {
        HanEthereum {
            local_swap_id: *swap.local_swap_id,
        }
    }
}

impl From<QueryableSwap> for HanBitcoin {
    fn from(swap: QueryableSwap) -> HanBitcoin {
        HanBitcoin {
            local_swap_id: *swap.local_swap_id,
        }
    }
}
