use crate::swap_protocols::SwapId;

mod han_han_swap;
mod integration_test;
mod new_save;

#[derive(Clone, Debug, PartialEq)]
pub struct NewSwap {
    pub local_swap_id: SwapId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HanBitcoin {
    pub local_swap_id: SwapId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HanEthereum {
    pub local_swap_id: SwapId,
}
