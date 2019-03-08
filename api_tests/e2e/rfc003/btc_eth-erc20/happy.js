const chai = require("chai");
const utils = require("web3-utils");
chai.use(require("chai-http"));
const bitcoin = require("../../../lib/bitcoin.js");
const actor = require("../../../lib/actor.js");
const ethutil = require("ethereumjs-util");
const ethereum = require("../../../lib/ethereum.js");
const should = chai.should();
const wallet = require("../../../lib/wallet.js");

const toby_wallet = wallet.create("toby", {
    ethConfig: global.harness.ledgers_config.ethereum,
});

const toby_initial_eth = "10";
const bob_initial_eth = utils.toBN(5);
const bob_initial_erc20 = utils.toBN(utils.toWei("10000", "ether"));

const alice = actor.create("alice", {
    ethConfig: global.harness.ledgers_config.ethereum,
});
const bob = actor.create("bob", {
    ethConfig: global.harness.ledgers_config.ethereum,
});

const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
const bob_final_address =
    "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";
const bob_comit_node_address = bob.config.comit.comit_listen;

const alpha_asset_quantity = 100000000;
const beta_asset_quantity = utils.toBN(utils.toWei("5000", "ether"));
const alpha_max_fee = 5000; // Max 5000 satoshis fee

const alpha_expiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
const beta_expiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

describe("RFC003: Bitcoin for ERC20", () => {
    let token_contract_address;
    before(async function() {
        this.timeout(5000);
        await bitcoin.ensureSegwit();
        await toby_wallet.eth().fund(toby_initial_eth);
        await bob.wallet.eth().fund(bob_initial_eth);
        await alice.wallet.btc().fund(10);
        await alice.wallet.eth().fund(utils.toBN(1));
        let receipt = await toby_wallet
            .eth()
            .deploy_erc20_token_contract(global.harness.project_root);
        token_contract_address = receipt.contractAddress;
        await bitcoin.generate();
    });

    it(bob_initial_erc20 + " tokens were minted to Bob", async function() {
        let bob_wallet_address = bob.wallet.eth().address();

        let receipt = await ethereum.mintErc20Tokens(
            toby_wallet.eth(),
            token_contract_address,
            bob_wallet_address,
            bob_initial_erc20
        );

        receipt.status.should.equal(true);

        let erc20_balance = await ethereum.erc20_balance(
            bob_wallet_address,
            token_contract_address
        );

        erc20_balance.eq(bob_initial_erc20).should.equal(true);
    });

    let swap_location;
    let alice_swap_href;

    it("[Alice] Should be able to make a swap request via HTTP api", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003")
            .send({
                alpha_ledger: {
                    name: "Bitcoin",
                    network: "regtest",
                },
                beta_ledger: {
                    name: "Ethereum",
                    network: "regtest",
                },
                alpha_asset: {
                    name: "Bitcoin",
                    quantity: alpha_asset_quantity.toString(),
                },
                beta_asset: {
                    name: "ERC20",
                    quantity: beta_asset_quantity.toString(),
                    token_contract: token_contract_address,
                },
                beta_ledger_redeem_identity: alice_final_address,
                alpha_expiry: alpha_expiry,
                beta_expiry: beta_expiry,
                peer: bob_comit_node_address,
            });

        res.should.have.status(201);
        swap_location = res.headers.location;
        swap_location.should.be.a("string");
        alice_swap_href = swap_location;
    });

    it("[Alice] Should be in IN_PROGRESS and SENT after sending the swap request to Bob", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            body =>
                body.status === "IN_PROGRESS" &&
                body.state.communication.status === "SENT"
        );
    });

    let bob_swap_href;

    it("[Bob] Shows the Swap as IN_PROGRESS in /swaps", async () => {
        let body = await bob.poll_comit_node_until(
            chai,
            "/swaps",
            body => body._embedded.swaps.length > 0
        );

        let swap_embedded = body._embedded.swaps[0];
        swap_embedded.protocol.should.equal("rfc003");
        swap_embedded.status.should.equal("IN_PROGRESS");
        let swap_link = swap_embedded._links;
        swap_link.should.be.a("object");
        bob_swap_href = swap_link.self.href;
        bob_swap_href.should.be.a("string");
    });

    let bob_accept_href;

    it("[Bob] Can get the accept action after Alice sends the swap request", async function() {
        this.timeout(10000);
        let body = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            body => body._links.accept && body._links.decline
        );
        bob_accept_href = body._links.accept.href;
    });

    it("[Bob] Can execute the accept action", async () => {
        let bob_response = {
            beta_ledger_refund_identity: bob.wallet.eth().address(),
            alpha_ledger_redeem_identity: null,
        };

        let accept_res = await chai
            .request(bob.comit_node_url())
            .post(bob_accept_href)
            .send(bob_response);

        accept_res.should.have.status(200);
    });

    let alice_fund_action;

    it("[Alice] Can get the fund action after Bob accepts", async function() {
        this.timeout(10000);
        let body = await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            body => body._links.fund
        );
        let alice_fund_href = body._links.fund.href;
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_fund_href);
        res.should.have.status(200);
        alice_fund_action = res.body;
    });

    it("[Alice] Can execute the fund action", async () => {
        alice_fund_action.payload.should.include.all.keys(
            "to",
            "amount",
            "network"
        );
        await alice.do(alice_fund_action);
    });

    let bob_deploy_action;

    it("[Bob] Can get the deploy action after Alice funds", async function() {
        this.timeout(10000);
        let body = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            body => body._links.deploy
        );
        let bob_deploy_href = body._links.deploy.href;
        let res = await chai.request(bob.comit_node_url()).get(bob_deploy_href);
        res.should.have.status(200);
        bob_deploy_action = res.body;
    });

    it("[Bob] Can execute the deploy action", async () => {
        bob_deploy_action.payload.should.include.all.keys(
            "data",
            "amount",
            "gas_limit",
            "network"
        );
        bob_deploy_action.payload.amount.should.equal("0");
        await bob.do(bob_deploy_action);
    });

    let bob_fund_action;

    it("[Bob] Can get the fund action after he deploys", async function() {
        this.timeout(10000);
        let body = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            body => body._links.fund
        );
        let bob_fund_href = body._links.fund.href;
        let res = await chai.request(bob.comit_node_url()).get(bob_fund_href);
        res.should.have.status(200);
        bob_fund_action = res.body;
    });

    it("[Bob] Can execute the fund action", async () => {
        bob_fund_action.payload.should.include.all.keys(
            "contract_address",
            "data",
            "amount",
            "gas_limit",
            "network"
        );
        let receipt = await bob.do(bob_fund_action);
        receipt.status.should.equal(true);
    });

    let alice_redeem_action;

    it("[Alice] Can get the redeem action after Bob funds", async function() {
        this.timeout(10000);
        let body = await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            body => body._links.redeem
        );
        let alice_redeem_href = body._links.redeem.href;
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_redeem_href);
        res.should.have.status(200);
        alice_redeem_action = res.body;
    });

    let alice_erc20_balance_before;

    it("[Alice] Can execute the redeem action", async function() {
        alice_redeem_action.payload.should.include.all.keys(
            "contract_address",
            "data",
            "amount",
            "gas_limit",
            "network"
        );
        alice_erc20_balance_before = await ethereum.erc20_balance(
            alice_final_address,
            token_contract_address
        );
        await alice.do(alice_redeem_action);
    });

    it("[Alice] Should have received the beta asset after the redeem", async function() {
        let alice_erc20_balance_after = await ethereum.erc20_balance(
            alice_final_address,
            token_contract_address
        );

        let alice_erc20_balance_expected = alice_erc20_balance_before.add(
            beta_asset_quantity
        );
        alice_erc20_balance_after
            .eq(alice_erc20_balance_expected)
            .should.equal(true);
    });

    let bob_redeem_action;

    it("[Bob] Can get the redeem action after Alice redeems", async function() {
        this.timeout(10000);
        let body = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            body => body._links.redeem
        );
        let bob_redeem_href = body._links.redeem.href;
        let res = await chai
            .request(bob.comit_node_url())
            .get(
                bob_redeem_href +
                    "?address=" +
                    bob_final_address +
                    "&fee_per_byte=20"
            );
        res.should.have.status(200);
        bob_redeem_action = res.body;
    });

    it("[Bob] Can execute the redeem action", async function() {
        bob_redeem_action.payload.should.include.all.keys("hex", "network");
        await bob.do(bob_redeem_action);
        await bitcoin.generate();
    });

    it("[Bob] Should have received the alpha asset after the redeem", async function() {
        this.timeout(10000);
        let body = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            body => body.state.alpha_ledger.status === "Redeemed"
        );
        let bob_redeem_txid = body.state.alpha_ledger.redeem_tx;

        let bob_satoshi_received = await bitcoin.get_first_utxo_value_transferred_to(
            bob_redeem_txid,
            bob_final_address
        );
        const bob_satoshi_expected = alpha_asset_quantity - alpha_max_fee;

        bob_satoshi_received.should.be.at.least(bob_satoshi_expected);
    });
});