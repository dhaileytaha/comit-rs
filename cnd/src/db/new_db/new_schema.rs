table! {
    swaps {
        id -> Integer,
        local_swap_id -> Text,
        // shared_swap_id -> Text,
        // role -> Text,
        // counter_party -> Text,
        // secret -> Text,
        // secret_hash -> Text,
   }
}
table! {
    han_bitcoin {
        id -> Integer,
        local_swap_id -> Text,
        // network -> Text,
        // amount -> Text,
        // hash_function -> Text,
        // refund_identity -> Text,
        // redeem_identity -> Text,
        // expiry -> BigInt,
   }
}

table! {
   han_ethereum {
        id -> Integer,
        local_swap_id -> Text,
        // chain_id -> BigInt,
        // amount -> Text,
        // hash_function -> Text,
        // refund_identity -> Text,
        // redeem_identity -> Text,
        // expiry -> BigInt,
   }
}
// table! {
//    herc20 {
//
//    }
// }
// table! {
//    halight {
//
//    }
// }

table! {
    swap_events {
        id -> Integer,
    }
}
table! {
   han_bitcoin_events {
        id -> Integer,
   }
}

table! {
   han_ethereum_events {
        id -> Integer,
   }
}
// table! {
//    herc20_events {
//
//    }
// }
//
// table! {
//    halight_events {
//
//    }
// }
