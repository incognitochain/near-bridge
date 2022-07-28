const nearAPI = require("near-api-js");
// creates keyStore from a private key string
// you can define your key here or use an environment variable

// creates keyStore from a private key string
// you can define your key here or use an environment variable

const { keyStores, KeyPair } = nearAPI;
const keyStore = new keyStores.InMemoryKeyStore();
const PRIVATE_KEY =
    "3Nd1XkAmVQPZ5d1znwkeictfBDPKTd67yP89zC93nVpu6ruY4f5RC7KTbB518KmWxhFNCdFMExn5Mgm8DwXLTFeb";
const PRIVATE_KEY_PROXY =
    "nKHv8jGLdTntaUf3Aa6xhYUNMCChUpyXDv19ZropeWwrjqcGioxP2kDkqFw4F2c8Cu2BzXd9zHFygeCMiNT2RUb";
const REMOVE_FRACTION = 1000000000000000n;
// creates a public / private key pair using the provided private key
const keyPair = KeyPair.fromString(PRIVATE_KEY);
const keyPairProxy = KeyPair.fromString(PRIVATE_KEY_PROXY);
console.log({keyPair});
const { connect } = nearAPI;
const Web3 = require("web3");
const web3 = new Web3();
(async () => {
    const pk58 = 'ed25519:5wbGqEmJuExCVCck6FLM5FqQRyyPabmBHpHtMbkZMUy1'
    const testAddress = nearAPI.utils.PublicKey.fromString(pk58).data.hexSlice();

    const pk58Proxy = 'ed25519:HGJpZM4DpovCuVFQiz8DFPowLPQ1XZaLbZe5j8aUs5s5'
    const testAddressProxy = nearAPI.utils.PublicKey.fromString(pk58Proxy).data.hexSlice();

    // adds the keyPair you created to keyStore
    await keyStore.setKey("testnet", testAddress, keyPair);
    await keyStore.setKey("testnet", testAddressProxy, keyPairProxy);

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
    const bridgeContractId = "0baceab06e95c52314f6792b2f5e6fd4ce5b583aeb63572f6a75bc56d820de66";
    const proxyContractId = "f1a6da2001ca6e98c2e4720619b413c882bd3e5d5e7997fc7dec345279ad10c8";
    const refContractId = "ref-finance-101.testnet";
    const bridgeContract = new nearAPI.Contract(
        account, // the account object that is connecting
        bridgeContractId,
        {
            // name of contract you're connecting to
            viewMethods: ["get_beacons", "get_tx_burn_used"], // view methods do not change state but usually return a value
            changeMethods: ["new", "deposit", "withdraw", "swap_beacon_committee", "execute_with_burn_proof"], // change methods modify state
            sender: account, // account object to initialize and sign transactions.
        }
    );

    const accountProxy = await near.account(testAddressProxy);
    const proxyContract = new nearAPI.Contract(
        accountProxy, // account object
        proxyContractId,
        {
            viewMethods: ["get_balance_token", "get_whitelisted_tokens"],
            changeMethods: ["deposit_near", "call_dapp", "extend_whitelisted_tokens"],
            sender: account,
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

    // white list token for proxy
    await proxyContract.extend_whitelisted_tokens(
        {
            args: {
                token_ids: ["wrap.testnet", "usdc.fakes.testnet"]
            },
            gas: "300000000000000",
            amount: "0"
        }
    );
    var whitelisted_tokens = await proxyContract.get_whitelisted_tokens();
    console.log(whitelisted_tokens);

    // register proxy contract to ref finance
    const refContract = new nearAPI.Contract(
        accountProxy, // account object
        refContractId,
        {
            changeMethods: ["storage_deposit"],
            sender: account,
        }
    );

    await refContract.storage_deposit(
        {
            account_id: proxyContractId,
            registration_only: false,
        },
        "300000000000000",
        "130000000000000000000000"
    );

    // build external call data
    const utf8Encode = new TextEncoder();
    let sourceToken = "wrap.testnet";
    let destToken = "usdc.fakes.testnet";
    let amount = "100000000000000000000";
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
    let meta = (160).toString(16).padStart(2, "0");
    let shardId = (1).toString(16).padStart(2, "0");
    let network = (1).toString(16).padStart(2, "0");
    let extToken = toHexString(utf8Encode.encode(sourceToken)).padStart(128, "0");
    let txId = getRanHex(64);
    let withdrawAddrNear = "";
    let withdrawAddr = toHexString(utf8Encode.encode(withdrawAddrNear)).padStart(128, "0");;
    let amountInst = web3.utils.numberToHex((BigInt(amount) / REMOVE_FRACTION).toString()).split("x")[1].padStart(64, "0");
    let redepositAddress = toHexString(utf8Encode.encode("12svfkP6w5UDJDSCwqH978PvqiqBxKmUnA9em9yAYWYJVRv7wuXY1qhhYpPAm4BDz2mLbFrRmdK3yRhnTqJCZXKHUmoi7NV83HCH2"));
    let extCallData = toHexString(utf8Encode.encode(JSON.stringify(obj)));

    // layout: meta(1), shard(1), network(1), len(1), extToken(64), amount(32), txID(32), len(1), withdrawAddr(64), redepositAddr(101), extCalldata(*)
    const beaconInst = meta + shardId + network + toHex(utf8Encode.encode(sourceToken).length) + extToken + amountInst
        + txId + toHex(withdrawAddrNear.length) + withdrawAddr + redepositAddress + extCallData;
    console.log({beaconInst});

    // call execute burn proof
    let unshieldInfo = {
        inst: beaconInst,
        height: 304,
        inst_paths: [
            to32Bytes("82a8c4d7dcdcf1e28ec58e7218155c8f2e75cdc2aded968d63da53efb8848abb"),
        ],
        inst_path_is_lefts: [
            false,
        ],
        inst_root: to32Bytes("fd64d3bd7f578bbb58ee9088949d96f4186b04b3d4b5751ce0104399d7ba4b7c"),
        blk_data: to32Bytes("b2f85d2ee41b2fc42a7e06dc90ca5ddec6d3b08e84a97519c9f9709315155681"),
        indexes: [1, 2, 3],
        signatures: [
            "af503e8cc61c73d6ae5728d5b37d04ec7fa7aff190040cbcffc213c7e2046e721a8d3fe9c12e62e9802f1bedaf1d38b58ea11ed2e38ec4301dd798c5be4ae469",
            "869f5225d790484190ec4bf5113dadc568e8232c90bb711909f3154ff54b477c3b96040e5d041e76dccad008c6453fdfbee8295119bf641f7b9bee8e0d79aa6d",
            "c3622371e7355ab5e4f48097e0e39b435b444f0557b6269c55cd427aac932fb327f5592748f839bbb98666613ebcb1e7100ea02f26a2becb960ba1ef8d909460",
        ],
        vs: [1, 0, 0],
    };

    // call to smart contract
    await bridgeContract.execute_with_burn_proof(
        {
            args: {
                burn_info: unshieldInfo
            },
            gas: "300000000000000",
            amount: "0"
        }
    );

})();

function toHex(number) {
    return ((number).toString(16).length % 2) === 0 ? (number).toString(16) : '0' + (number).toString(16);
}

function toHexString(byteArray) {
    return Array.from(byteArray, function(byte) {
        return ('0' + (byte & 0xFF).toString(16)).slice(-2);
    }).join('')
}

function to32Bytes(hexStr) {
    const bytes = Buffer.from(hexStr, "hex");
    const padded = Buffer.alloc(32);
    bytes.copy(padded);
    const arr = [...padded];
    return arr;
}

const getRanHex = size => {
    let result = [];
    let hexRef = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'];

    for (let n = 0; n < size; n++) {
        result.push(hexRef[Math.floor(Math.random() * 16)]);
    }
    return result.join('');
}