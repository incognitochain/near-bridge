const nearAPI = require("near-api-js");
const fs = require('fs');
// creates keyStore from a private key string
// you can define your key here or use an environment variable

// creates keyStore from a private key string
// you can define your key here or use an environment variable

const { keyStores, KeyPair } = nearAPI;
const keyStore = new keyStores.InMemoryKeyStore();
const PRIVATE_KEY =
    "3Nd1XkAmVQPZ5d1znwkeictfBDPKTd67yP89zC93nVpu6ruY4f5RC7KTbB518KmWxhFNCdFMExn5Mgm8DwXLTFeb";
// creates a public / private key pair using the provided private key
const keyPair = KeyPair.fromString(PRIVATE_KEY);
console.log({keyPair});
const { connect } = nearAPI;
const { toHexString } = require('./shieldtests');
const { web3 } = require('web3');

(async () => {
    const pk58 = 'ed25519:5wbGqEmJuExCVCck6FLM5FqQRyyPabmBHpHtMbkZMUy1'
    const testAddress = nearAPI.utils.PublicKey.fromString(pk58).data.hexSlice();

    // adds the keyPair you created to keyStore
    await keyStore.setKey("testnet", testAddress, keyPair);
    const config = {
        networkId: "testnet",
        keyStore,
        nodeUrl: "https://rpc.testnet.near.org",
        walletUrl: "https://wallet.testnet.near.org",
        helperUrl: "https://helper.testnet.near.org",
        explorerUrl: "https://explorer.testnet.near.org",
    };
    const near = await connect(config);
    const account = await near.account(testAddress);
    let balance = await account.getAccountBalance();

    // const response = await account.deployContract(fs.readFileSync('../target/wasm32-unknown-unknown/release/bridge.wasm'));
    // console.log(response);
    // const contractId = response.transaction_outcome.outcome.executor_id;
    const bridgeContractId = "496add2c24e17711d9512172901b5502df37e10493d247c371eb8dc3e4b173fc";
    const proxyContractId = "496add2c24e17711d9512172901b5502df37e10493d247c371eb8dc3e4b173fc";
    const contract = new nearAPI.Contract(
        account, // the account object that is connecting
        bridgeContractId,
        {
            // name of contract you're connecting to
            viewMethods: ["get_beacons", "get_tx_burn_used"], // view methods do not change state but usually return a value
            changeMethods: ["new", "deposit", "withdraw", "swap_beacon_committee", "execute_with_burn_proof"], // change methods modify state
            sender: account, // account object to initialize and sign transactions.
        }
    );

    // make shield Near request
    const incognitoAddress = "12svfkP6w5UDJDSCwqH978PvqiqBxKmUnA9em9yAYWYJVRv7wuXY1qhhYpPAm4BDz2mLbFrRmdK3yRhnTqJCZXKHUmoi7NV83HCH2YFpctHNaDdkSiQshsjw2UFUuwdEvcidgaKmF3VJpY5f8RdN";
    // await contract.deposit(
    //     {
    //         args: {
    //             incognito_address: incognitoAddress
    //         },
    //         gas: "300000000000000",
    //         amount: "1000000000000000000000"
    //     },
    // );

    // todo:
    // make execute burn proof request
    // external call data
    const utf8Encode = new TextEncoder();
    let sourceToken = "wrap.testnet";
    let destToken = "usdc.fakes.testnet";
    let amount = "95904358992446227499";
    let obj = {
        action: {
            pool_id: 54,
            token_in: sourceToken,
            amount_in: amount,
            token_out: destToken,
            min_amount_out: "1",
        },
        account_id: testAddress,
    }
    let meta = (1).toString(16).padStart(2, "0");
    let shardId = (1).toString(16).padStart(2, "0");
    let network = (1).toString(16).padStart(2, "0");
    let extToken = toHexString(utf8Encode.encode("wrap.testnet")).padStart(64, "0");
    let txId = "0baceab06e95c52314f6792b2f5e6fd4ce5b583aeb63572f6a75bc56d820de66";
    let withdrawAddr = "";
    let amountInst = web3.utils.numberToHex(amount).slice(0, 2).split("x")[1].padStart(64, "0");
    let redepositAddress = toHexString(utf8Encode.encode("12svfkP6w5UDJDSCwqH978PvqiqBxKmUnA9em9yAYWYJVRv7wuXY1qhhYpPAm4BDz2mLbFrRmdK3yRhnTqJCZXKHUmoi7NV83HCH2YFpctHNaDdkSiQshsjw2UFUuwdEvcidgaKmF3VJpY5f8RdN"));
    let extCallData = toHexString(utf8Encode.encode(JSON.stringify(obj)));

    // layout: meta(1), shard(1), network(1), len(1), extToken(32), amount(32), txID(32), len(1), withdrawAddr(32), redepositAddr(101), extCalldata(*)
    const beaconInst = meta + shardId + network + toHex(extToken.length) + extToken + txId +
        + toHex(withdrawAddr.length) + withdrawAddr + amountInst + redepositAddress + extCallData;
    console.log({beaconInst});

})();

function toHex(number) {
    return ((number).toString(16).length % 2) === 0 ? (number).toString(16) : '0' + (number).toString(16);
}