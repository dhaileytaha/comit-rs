use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{
            bitcoin::BitcoinRedeem,
            ethereum::{EtherDeploy, EtherRefund},
            Accept, Action, Decline, StateActions,
        },
        ethereum::{EtherHtlc, Htlc},
        roles::Bob,
        state_machine::*,
    },
};

impl StateActions for SwapStates<Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>> {
    type Accept = Accept;
    type Decline = Decline;
    type Fund = EtherDeploy;
    type Redeem = BitcoinRedeem;
    type Refund = EtherRefund;

    fn actions(&self) -> Vec<Action<Accept, Decline, EtherDeploy, BitcoinRedeem, EtherRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![Action::Accept(Accept), Action::Decline(Decline)],
            SS::Accepted { .. } => vec![],
            SS::SourceFunded(SourceFunded { ref swap, .. }) => {
                let htlc: EtherHtlc = swap.target_htlc_params().into();
                vec![Action::Fund(EtherDeploy {
                    data: htlc.compile_to_hex().into(),
                    value: swap.target_asset,
                    gas_limit: 42.into(), //TODO come up with correct gas limit
                    gas_cost: 42.into(),  //TODO come up with correct gas cost
                })]
            }
            SS::BothFunded(BothFunded {
                ref target_htlc_location,
                ..
            }) => vec![Action::Refund(EtherRefund {
                contract_address: *target_htlc_location,
                gas_limit: 42.into(), //TODO come up with correct gas_limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::SourceFundedTargetRefunded { .. } => vec![],
            SS::SourceRedeemedTargetFunded(SourceRedeemedTargetFunded {
                ref target_htlc_location,
                ..
            }) => vec![Action::Refund(EtherRefund {
                contract_address: *target_htlc_location,
                gas_limit: 42.into(), //TODO come up with correct gas_limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::SourceRefundedTargetFunded(SourceRefundedTargetFunded {
                ref target_htlc_location,
                ..
            }) => vec![Action::Refund(EtherRefund {
                contract_address: *target_htlc_location,
                gas_limit: 42.into(), //TODO come up with correct gas_limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::SourceFundedTargetRedeemed(SourceFundedTargetRedeemed {
                ref swap,
                ref source_htlc_location,
                ref secret,
                ..
            }) => vec![Action::Redeem(BitcoinRedeem {
                outpoint: *source_htlc_location,
                htlc: swap.source_htlc_params().into(),
                value: swap.source_asset,
                transient_keypair: swap.source_ledger_success_identity,
                secret: *secret,
            })],
            SS::Error(_) => vec![],
            SS::Final(_) => vec![],
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use bitcoin_support;
    use hex::FromHex;
    use swap_protocols::rfc003::{roles::test::Bobisha, Secret};

    #[test]
    fn given_state_instance_when_calling_actions_should_not_need_to_specify_type_arguments() {
        let swap_state = SwapStates::from(Start::<Bobisha> {
            source_ledger_refund_identity: bitcoin_support::PubkeyHash::from_hex(
                "875638cac0b0ae9f826575e190f2788918c354c2",
            )
            .unwrap(),
            target_ledger_success_identity: "8457037fcd80a8650c4692d7fcfc1d0a96b92867"
                .parse()
                .unwrap(),
            source_ledger: Bitcoin::regtest(),
            target_ledger: Ethereum::default(),
            source_asset: BitcoinQuantity::from_bitcoin(1.0),
            target_asset: EtherQuantity::from_eth(10.0),
            source_ledger_lock_duration: bitcoin_support::Blocks::from(144),
            secret: Secret::from(*b"hello world, you are beautiful!!").hash(),
        });

        let actions = swap_state.actions();

        assert_eq!(
            actions,
            vec![Action::Accept(Accept), Action::Decline(Decline)]
        );
    }

}
