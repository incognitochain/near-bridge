const nearAPI = require("near-api-js");
const fs = require('fs');
// creates keyStore from a private key string
// you can define your key here or use an environment variable

// creates keyStore from a private key string
// you can define your key here or use an environment variable

const { keyStores, KeyPair } = nearAPI;
const keyStore = new keyStores.InMemoryKeyStore();
const PRIVATE_KEY =
    "4GrZBkRSEp8YT6ztXHUu9wzrDb3qrpFpyTzEsFR5yovjbGqt16aKQVR7WHoMUdBoNwe2NJRGZ22mt1o3j2wda1jk";
const SENDER_ADDRESS = "cuongcute.testnet";
const TOKEN_ADDRESS = "ft0.cuongcute.testnet"
// creates a public / private key pair using the provided private key
const keyPair = KeyPair.fromString(PRIVATE_KEY);
console.log({keyPair});
const { connect } = nearAPI;

(async () => {
    // adds the keyPair you created to keyStore
    await keyStore.setKey("testnet", SENDER_ADDRESS, keyPair);

    const config = {
        networkId: "testnet",
        keyStore,
        nodeUrl: "https://rpc.testnet.near.org",
        walletUrl: "https://wallet.testnet.near.org",
        helperUrl: "https://helper.testnet.near.org",
        explorerUrl: "https://explorer.testnet.near.org",
    };
    const near = await connect(config);
    const senderAccount = await near.account(SENDER_ADDRESS)
    // const tokenAccount = await near.account(TOKEN_ADDRESS)

    

    console.log({senderAddress: senderAccount});

    const contractId = "496add2c24e17711d9512172901b5502df37e10493d247c371eb8dc3e4b173fc";
    console.log(contractId);

    const contract = new nearAPI.Contract(
        senderAccount, // account object đang kết nối
        TOKEN_ADDRESS,
        {
          changeMethods: ["ft_transfer_call", "ft_transfer", "storage_deposit"], 
          sender: senderAccount,
        }
      );
      
    // register account id
    await contract.storage_deposit(
        {
            account_id: "prv-test-shield.testnet",
            registration_only: true,
        },
        "300000000000000",
        "130000000000000000000000"
    );

    // make shield Near request
    const incognitoAddress = "12svfkP6w5UDJDSCwqH978PvqiqBxKmUnA9em9yAYWYJVRv7wuXY1qhhYpPAm4BDz2mLbFrRmdK3yRhnTqJCZXKHUmoi7NV83HCH2YFpctHNaDdkSiQshsjw2UFUuwdEvcidgaKmF3VJpY5f8RdN";
    await contract.ft_transfer_call(
        {
            sender_id: "cuongcute.testnet",
            receiver_id: contractId,
            amount: "10000000000",
            msg: '{"incognito_address": "' + incognitoAddress + '"}'
        },
        "300000000000000",
        "1"
    );

})();

function toHexString(byteArray) {
    return Array.from(byteArray, function(byte) {
        return ('0' + (byte & 0xFF).toString(16)).slice(-2);
    }).join('')
}