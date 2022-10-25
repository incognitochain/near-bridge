const nearAPI = require("near-api-js");
const fs = require('fs');
// creates keyStore from a private key string
// you can define your key here or use an environment variable

// creates keyStore from a private key string
// you can define your key here or use an environment variable

const { keyStores, KeyPair, connect } = nearAPI;
const keyStore = new keyStores.InMemoryKeyStore();

(async () => {
    const testAddress = "inc.prv.testnet";
    const PRIVATE_KEY_TESTNET =
        "2EZvXozYRirEoGJKEraa4fRsCZm3wm997gKvkqFiLEeuWK5HGcaXm776V7UyGD37fJZK7vNdjZfHsPoivvCPjXao";
    // creates a public / private key pair using the provided private key
    const keyPair = KeyPair.fromString(PRIVATE_KEY_TESTNET);

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

    const connectionConfig = {
        networkId: "mainnet",
        keyStore,
        nodeUrl: "https://rpc.mainnet.near.org",
        walletUrl: "https://wallet.mainnet.near.org",
        helperUrl: "https://helper.mainnet.near.org",
        explorerUrl: "https://explorer.mainnet.near.org",
    };

    const address = "incognito_chain.near";
    const PRIVATE_KEY_MAINNET = "{}";
    // creates a public / private key pair using the provided private key
    const keyPair_main = KeyPair.fromString(PRIVATE_KEY_MAINNET);
    await keyStore.setKey("mainnet", address, keyPair_main);

    const near = await connect(connectionConfig);
    // const account = await near.account(testAddress);
    console.log({testAddress});
    const account = await near.account(address);
    // await account.createAccount(
    //     "inc.prv.testnet", // new account name
    //     "5Lx7Exo3VkSfYZ1pjFUKLYNFR6AmjAkmrL5NKWUDfNGi", // public key for new account
    //     "2000000000000000000000" // initial balance for new account in yoctoNEAR
    // );
    //
    let balance = await account.getAccountBalance();
    console.log({balance});

    const response = await account.deployContract(fs.readFileSync('../target/wasm32-unknown-unknown/release/bridge.wasm'));
    console.log(response);

})();