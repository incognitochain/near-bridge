const nearAPI = require("near-api-js");
const fs = require('fs');
const { wrap } = require("module");
// creates keyStore from a private key string
// you can define your key here or use an environment variable

// creates keyStore from a private key string
// you can define your key here or use an environment variable

const { keyStores, KeyPair } = nearAPI;
const keyStore = new keyStores.InMemoryKeyStore();
const PRIVATE_KEY =
    "3FjKs6g4s6vSf2QaUWKMaRm7ATKKN6CZF4GUpHgAhnZkKTAykCrCSwQ4zsphgBSGTQCmhV3nwoFCoRMeiFhkkKjC";
// creates a public / private key pair using the provided private key
const keyPair = KeyPair.fromString(PRIVATE_KEY);
console.log({ keyPair });
const { connect } = nearAPI;

(async () => {
    const bridgeAddress = "bridge.incognito_chain.testnet";
    const testAddress = "cuongcute.testnet"

    // adds the keyPair you created to keyStore
    await keyStore.setKey("testnet", bridgeAddress, keyPair);
    const config = {
        networkId: "testnet",
        keyStore,
        nodeUrl: "https://rpc.testnet.near.org",
        walletUrl: "https://wallet.testnet.near.org",
        helperUrl: "https://helper.testnet.near.org",
        explorerUrl: "https://explorer.testnet.near.org",
    };
    const near = await connect(config);
    const account = await near.account(bridgeAddress);
    console.log({ testAddress: bridgeAddress });


    const proxyAddress = "proxy1.incognito_chain.testnet";

    const contract = new nearAPI.Contract(
        account, // account object
        proxyAddress,
        {
            viewMethods: ["get_balance_token", "get_whitelisted_tokens"],
            changeMethods: ["deposit_near", "call_dapp", "extend_whitelisted_tokens"],
            sender: account,
        }
    );

    await contract.extend_whitelisted_tokens(
            {
            args: {
                token_ids: ["wrap.testnet", "usdc.fakes.testnet"]
            },
            gas: "300000000000000",
            amount: "0"
        }
    )
    var whitelisted_tokens = await contract.get_whitelisted_tokens()
    console.log(whitelisted_tokens)

    const wrapContract = new nearAPI.Contract(
        account, // account object
        "wrap.testnet",
        {
            viewMethods: ["ft_balance_of"],
            sender: account,
        }
    );
    const usdcContract = new nearAPI.Contract(
        account, // account object
        "usdc.fakes.testnet",
        {
            viewMethods: ["ft_balance_of"],
            sender: account,
        }
    );

    // deposit NEAR and wrap
    // await contract.deposit_near(
    //     {
    //         args: {
    //             account_id: testAddress,
    //             wrap: true,
    //         },
    //         gas: "300000000000000",
    //         amount: "1000000000000000000000"
    //     },
    // );

    var balance = await contract.get_balance_token({
        account_id: testAddress,
        token_id: "wrap.testnet",
    });
    console.log("wrap balance: " + balance);


    var balance = await contract.get_balance_token({
        account_id: testAddress,
        token_id: "usdc.fakes.testnet",
    });
    console.log("usdc balance: " + balance);

    const proxyBalance = await wrapContract.ft_balance_of({
        account_id: proxyAddress,
    })
    console.log({ proxyBalance })

    var obj = {
        action: {
            pool_id: 54,
            token_in: "wrap.testnet",
            amount_in: "95904358992446227499",
            token_out: "usdc.fakes.testnet",
            min_amount_out: "157",
        },
        account_id: testAddress,
    }
    var msg = JSON.stringify(obj)

    // trade success
    await contract.call_dapp(
        {
            args: {
                msg: msg,
            },
            gas: "300000000000000",
            amount: "0"
        }
    );

    var obj = {
        action: {
            pool_id: 54,
            token_in: "wrap.testnet",
            amount_in: "95904358992446227499",
            token_out: "usdc.fakes.testnet",
            min_amount_out: "15700",
        },
        account_id: testAddress,
    }
    var msg = JSON.stringify(obj)

    // trade fail
    await contract.call_dapp(
        {
            args: {
                msg: msg,
            },
            gas: "300000000000000",
            amount: "0"
        }
    );
})();