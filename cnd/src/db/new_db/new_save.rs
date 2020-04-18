use crate::{
    db::{
        new_db::{HanBitcoin, HanEthereum, NewSwap},
        schema::{self, *},
        wrapper_types::custom_sql_types::Text,
        Sqlite,
    },
    swap_protocols::SwapId,
};
use async_trait::async_trait;
use diesel::RunQueryDsl;

#[async_trait]
pub trait Save<T, A, B>: Send + Sync + 'static {
    async fn save(&self, swap: T, alpha: A, beta: B) -> anyhow::Result<()>;
}

#[async_trait]
impl Save<NewSwap, HanBitcoin, HanEthereum> for Sqlite {
    async fn save(
        &self,
        swap: NewSwap,
        alpha: HanBitcoin,
        beta: HanEthereum,
    ) -> anyhow::Result<()> {
        let insertable_swap = InsertableSwap::from(swap);
        let insertable_alpha = InsertableHanBitcoin::from(alpha);
        let insertable_beta = InsertableHanEthereum::from(beta);

        self.do_in_transaction(|connection| {
            diesel::insert_into(schema::swaps::dsl::swaps)
                .values(&insertable_swap)
                .execute(&*connection)
        })
        .await?;

        self.do_in_transaction(|connection| {
            diesel::insert_into(schema::han_bitcoin::dsl::han_bitcoin)
                .values(&insertable_alpha)
                .execute(&*connection)
        })
        .await?;

        self.do_in_transaction(|connection| {
            diesel::insert_into(schema::han_ethereum::dsl::han_ethereum)
                .values(&insertable_beta)
                .execute(&*connection)
        })
        .await?;

        Ok(())
    }
}

#[derive(Identifiable, Insertable, Debug, Clone)]
#[table_name = "swaps"]
#[primary_key(local_swap_id)]
struct InsertableSwap {
    pub local_swap_id: Text<SwapId>,
}

#[derive(Associations, Identifiable, Insertable, Debug, Clone)]
#[belongs_to(parent = "InsertableSwap", foreign_key = "local_swap_id")]
#[table_name = "han_bitcoin"]
#[primary_key(local_swap_id)]
struct InsertableHanBitcoin {
    pub local_swap_id: Text<SwapId>,
}

#[derive(Associations, Identifiable, Insertable, Debug, Clone)]
#[belongs_to(parent = "InsertableSwap", foreign_key = "local_swap_id")]
#[table_name = "han_ethereum"]
#[primary_key(local_swap_id)]
struct InsertableHanEthereum {
    pub local_swap_id: Text<SwapId>,
}

impl From<NewSwap> for InsertableSwap {
    fn from(swap: NewSwap) -> Self {
        InsertableSwap {
            local_swap_id: Text(swap.local_swap_id),
        }
    }
}
impl From<HanBitcoin> for InsertableHanBitcoin {
    fn from(han_bitcoin: HanBitcoin) -> Self {
        InsertableHanBitcoin {
            local_swap_id: Text(han_bitcoin.local_swap_id),
        }
    }
}

impl From<HanEthereum> for InsertableHanEthereum {
    fn from(han_ethereum: HanEthereum) -> Self {
        InsertableHanEthereum {
            local_swap_id: Text(han_ethereum.local_swap_id),
        }
    }
}
