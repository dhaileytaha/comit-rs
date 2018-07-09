extern crate bitcoin;
extern crate bitcoin_node;
extern crate bitcoin_rpc;
extern crate bitcoin_support;
extern crate bitcoin_witness;
extern crate hex;
extern crate secp256k1_support;

use bitcoin_node::BitcoinNode;
use bitcoin_rpc::{BitcoinRpcApi, TestUtility, TransactionId};
use bitcoin_support::serialize::serialize_hex;
use bitcoin_support::{Address, BitcoinQuantity, Network, PrivateKey, ToP2wpkhAddress};
use bitcoin_witness::{PrimedInput, PrimedTransaction, WitnessP2pkh};
use secp256k1_support::ToPublicKey;
use std::str::FromStr;

fn create_vout<C: BitcoinRpcApi>(
    client: &C,
    private_key: &PrivateKey,
    amount: BitcoinQuantity,
) -> (TransactionId, u32) {
    let address = private_key
        .secret_key()
        .to_public_key()
        .to_p2wpkh_address(Network::BitcoinCoreRegtest);

    let txid = client
        .send_to_address(&address.clone().into(), amount.bitcoin())
        .unwrap()
        .into_result()
        .unwrap();

    client.generate(1).unwrap();

    let vout = client.find_vout_for_address(&txid, &address);

    (txid, vout.n)
}

#[test]
fn redeem_single_p2wpkh() {
    let bitcoin_node = BitcoinNode::new();
    let client = bitcoin_node.get_client();
    client.enable_segwit();
    let input_amount = BitcoinQuantity::from_satoshi(100_000_001);
    let private_key =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let (txid, vout) = create_vout(client, &private_key, input_amount);

    let alice_addr: Address = client
        .get_new_address()
        .unwrap()
        .into_result()
        .unwrap()
        .into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![
            PrimedInput::new(
                txid.into(),
                vout,
                input_amount,
                WitnessP2pkh(private_key.secret_key().clone()),
            ),
        ],
        output_address: alice_addr.clone(),
        locktime: 0,
    }.sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx).unwrap();

    let raw_redeem_tx = bitcoin_rpc::SerializedRawTransaction::from(redeem_tx_hex.as_str());

    let rpc_redeem_txid = client
        .send_raw_transaction(raw_redeem_tx.clone())
        .unwrap()
        .into_result()
        .unwrap();

    client.generate(1).unwrap();

    assert_eq!(
        client
            .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
            .unwrap()
            .amount,
        (input_amount - fee).bitcoin(),
        "utxo should exist after redeeming p2wpkhoutput"
    );
}

#[test]
fn redeem_two_p2wpkh() {
    let bitcoin_node = BitcoinNode::new();
    let client = bitcoin_node.get_client();
    client.enable_segwit();
    let input_amount = BitcoinQuantity::from_satoshi(100_000_001);
    let private_key_1 =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let private_key_2 =
        PrivateKey::from_str("L1dDXCRQuNuhinf5SHbAmNUncovqFdA6ozJP4mbT7Mg53tWFFMFL").unwrap();

    let (txid_1, vout_1) = create_vout(client, &private_key_1, input_amount);
    let (txid_2, vout_2) = create_vout(client, &private_key_2, input_amount);

    let alice_addr: Address = client
        .get_new_address()
        .unwrap()
        .into_result()
        .unwrap()
        .into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![
            PrimedInput::new(
                txid_1.into(),
                vout_1,
                input_amount,
                WitnessP2pkh(private_key_1.secret_key().clone()),
            ),
            PrimedInput::new(
                txid_2.into(),
                vout_2,
                input_amount,
                WitnessP2pkh(private_key_2.secret_key().clone()),
            ),
        ],
        output_address: alice_addr.clone(),
        locktime: 0,
    }.sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx).unwrap();

    let raw_redeem_tx = bitcoin_rpc::SerializedRawTransaction::from(redeem_tx_hex.as_str());

    let rpc_redeem_txid = client
        .send_raw_transaction(raw_redeem_tx.clone())
        .unwrap()
        .into_result()
        .unwrap();

    client.generate(1).unwrap();

    assert_eq!(
        client
            .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
            .unwrap()
            .amount,
        BitcoinQuantity::from_satoshi(input_amount.satoshi() * 2 - fee.satoshi()).bitcoin(),
        "The utxo should include the amounts from both inputs"
    );
}

#[test]
fn sign_with_rate() {
    let bitcoin_node = BitcoinNode::new();
    let client = bitcoin_node.get_client();
    client.enable_segwit();
    let input_amount = BitcoinQuantity::from_satoshi(100_000_001);
    let private_key =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let (txid, vout) = create_vout(client, &private_key, input_amount);

    let alice_addr: Address = client
        .get_new_address()
        .unwrap()
        .into_result()
        .unwrap()
        .into();

    let rate = BitcoinQuantity::from_satoshi(42);

    let primed_tx = PrimedTransaction {
        inputs: vec![
            PrimedInput::new(
                txid.into(),
                vout,
                input_amount,
                WitnessP2pkh(private_key.secret_key().clone()),
            ),
        ],
        output_address: alice_addr.clone(),
        locktime: 0,
    };

    let redeem_tx = primed_tx.sign_with_rate(rate);

    let redeem_tx_hex = serialize_hex(&redeem_tx).unwrap();

    let raw_redeem_tx = bitcoin_rpc::SerializedRawTransaction::from(redeem_tx_hex.as_str());

    let rpc_redeem_txid = client
        .send_raw_transaction(raw_redeem_tx.clone())
        .unwrap()
        .into_result()
        .unwrap();

    client.generate(1).unwrap();

    assert!(
        client
            .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
            .is_some()
    )
}
