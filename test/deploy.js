const nearAPI = require("near-api-js");
const fs = require('fs');
// creates keyStore from a private key string
// you can define your key here or use an environment variable

// creates keyStore from a private key string
// you can define your key here or use an environment variable

const { keyStores, KeyPair } = nearAPI;
const keyStore = new keyStores.InMemoryKeyStore();
const PRIVATE_KEY =
    "2EZvXozYRirEoGJKEraa4fRsCZm3wm997gKvkqFiLEeuWK5HGcaXm776V7UyGD37fJZK7vNdjZfHsPoivvCPjXao";
// creates a public / private key pair using the provided private key
const keyPair = KeyPair.fromString(PRIVATE_KEY);
console.log({keyPair});
const { connect } = nearAPI;

(async () => {
    const pk58 = 'ed25519:GVNapxiWxGXuc1m8nftuvjj7394G2XGGtXGZmhxKZNgv'
    const testAddress = "near.bridge.incognito_chain.testnet";

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
    //     "near.bridge.incognito_chain.testnet", // new account name
    //     "DqFNkvBJtkbNG8b1DnPtSiDgVaG4TGeCsEzzwwWN9BNP", // public key for new account
    //     "2000000000000000000000" // initial balance for new account in yoctoNEAR
    // );

    let balance = await account.getAccountBalance();
    console.log({balance});

    const response = await account.deployContract(fs.readFileSync('../target/wasm32-unknown-unknown/release/bridge.wasm'));
    console.log(response);

})();