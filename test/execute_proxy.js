const nearAPI = require("near-api-js");
const fs = require('fs');
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


    const proxyAddress = "proxy0.incognito_chain.testnet";

    const contract = new nearAPI.Contract(
        account, // account object
        proxyAddress,
        {
            viewMethods: ["get_balance_token"],
            changeMethods: ["deposit_near", "withdraw"],
            sender: account,
        }
    );

    const tokenContract = new nearAPI.Contract(
        account, // account object
        "wrap.testnet",
        {
            viewMethods: ["ft_balance_of"],
            sender: account,
        }
    );

    // deposit NEAR and wrap
    await contract.deposit_near(
        {
            args: {
                account_id: testAddress,
                wrap: true,
            },
            gas: "300000000000000",
            amount: "1000000000000000000000"
        },
    );

    const balance = await contract.get_balance_token({
        account_id: testAddress,
        token_id: "wrap.testnet",
    });
    console.log({ balance });

    const proxyBalance = await tokenContract.ft_balance_of({
        account_id: proxyAddress,
    })
    console.log({ proxyBalance })

    // withdraw wNEAR
    await contract.withdraw(
        {
            args: {
                token_id: "",
                amount: 500000000000000000000,
                account_id: testAddress,
                incognito_address: "12svfkP6w5UDJDSCwqH978PvqiqBxKmUnA9em9yAYWYJVRv7wuXY1qhhYpPAm4BDz2mLbFrRmdK3yRhnTqJCZXKHUmoi7NV83HCH2YFpctHNaDdkSiQshsjw2UFUuwdEvcidgaKmF3VJpY5f8RdN",
            },
            gas: "300000000000000",
            amount: "0"
        }
    );

    // withdraw NEAR
    await contract.withdraw(
        {
            args: {
                token_id: "",
                amount: 400000000000000000000,
                account_id: testAddress,
                incognito_address: "12svfkP6w5UDJDSCwqH978PvqiqBxKmUnA9em9yAYWYJVRv7wuXY1qhhYpPAm4BDz2mLbFrRmdK3yRhnTqJCZXKHUmoi7NV83HCH2YFpctHNaDdkSiQshsjw2UFUuwdEvcidgaKmF3VJpY5f8RdN",
            },
            gas: "300000000000000",
            amount: "0"
        }
    );
})();