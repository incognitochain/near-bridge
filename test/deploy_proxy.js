const nearAPI = require("near-api-js");
const fs = require('fs');
// creates keyStore from a private key string
// you can define your key here or use an environment variable

// creates keyStore from a private key string
// you can define your key here or use an environment variable

const { keyStores, KeyPair } = nearAPI;
const keyStore = new keyStores.InMemoryKeyStore();
const PRIVATE_KEY =
    "eAcwDR12Fxyi4pRad7L1jgc4M7x8oJpmK6R3NkgQaDu2abVNm14fmeKT5kqvxS1T8FgCRDLWmjVQpKmcetDSZ5E";
// creates a public / private key pair using the provided private key
const keyPair = KeyPair.fromString(PRIVATE_KEY);
console.log({keyPair});
const { connect } = nearAPI;

(async () => {
    const testAddress = "proxy0.incognito_chain.testnet";

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
    console.log({testAddress});
    // const account = await near.account("incognito.bridge.testnet");
    // await account.createAccount(
    //     "example-account2.testnet", // new account name
    //     "8hSHprDq2StXwMtNd43wDTXQYsjXcD4MJTXQYsjXcc", // public key for new account
    //     "10000000000000000000" // initial balance for new account in yoctoNEAR
    // );

    let balance = await account.getAccountBalance();
    console.log({balance});

    const response = await account.deployContract(fs.readFileSync('../target/wasm32-unknown-unknown/release/proxy.wasm'));
    console.log(response);

    const contract = new nearAPI.Contract(
        account, // the account object that is connecting
        testAddress,
        {
            // name of contract you're connecting to
            changeMethods: ["new"], // change methods modify state
            sender: account, // account object to initialize and sign transactions.
        }
    );

    // init bridge contract
    await contract.new(
        {
            args: {},
            gas: "300000000000000",
            amount: "0"
        },
    );

})();