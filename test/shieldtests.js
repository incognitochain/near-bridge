const nearAPI = require("near-api-js");
const ethers = require('ethers');
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
    console.log({testAddress});

    let balance = await account.getAccountBalance();
    console.log({balance});

    const contractId = "incognito.prv.testnet";

    const beacon1 = "3cD69B1A595B7A9589391538d29ee7663326e4d3";
    const beacon2 = "c687470342f4E80ECEf6bBd25e276266d40b8429";
    const beacon3 = "2A40c96b41AdEc5641F28eF923e270B73e29bb53";
    const beacon4 = "131B772A9ADe1793F000024eAb23b77bEd3BFe64";

    const contract = new nearAPI.Contract(
        account, // the account object that is connecting
        contractId,
        {
            // name of contract you're connecting to
            viewMethods: ["get_beacons", "get_tx_burn_used"], // view methods do not change state but usually return a value
            changeMethods: ["new", "deposit", "withdraw", "swap_beacon_committee", "submit_burn_proof"], // change methods modify state
            sender: account, // account object to initialize and sign transactions.
        }
    );

    // init bridge contract
    // await contract.new(
    //     {
    //         args: {
    //             beacons: [
    //                 beacon1,
    //                 beacon2,
    //                 beacon3,
    //                 beacon4
    //             ],
    //             height: 0,
    //         },
    //         gas: "300000000000000",
    //         amount: "0"
    //     },
    // );

    const beaconlist = await contract.get_beacons({
        height: 0
    });
    console.log({beaconlist});

    // regulator key
    const hexPrivateKey = "0x98452cb9c013387c2f5806417fe198a0de014594678e2f9d3223d7e7e921b04d";
    const signingKey = new ethers.utils.SigningKey(hexPrivateKey);
    const tx = "65bQNcfAKdfLzZZFsW9KECnQ8JFADQFocMEtTapkEpbp";
    const shieldInfo = JSON.stringify(
        {
            sender: testAddress,
            tx,
        }
    ); //'{"sender":"incognito.deployer.testnet","tx":"65bQNcfAKdfLzZZFsW9KECnQ8JFADQFocMEtTapkEpbp"}';
    const signature = signingKey.signDigest(ethers.utils.id(shieldInfo));
    console.log({shieldInfo});
    console.log({"signature" : ethers.utils.joinSignature(signature).slice(0, -2) + '0' + signature.recoveryParam.toString()});

    // make shield Near request
    const incognitoAddress = "12svfkP6w5UDJDSCwqH978PvqiqBxKmUnA9em9yAYWYJVRv7wuXY1qhhYpPAm4BDz2mLbFrRmdK3yRhnTqJCZXKHUmoi7NV83HCH2YFpctHNaDdkSiQshsjw2UFUuwdEvcidgaKmF3VJpY5f8RdN";
    await contract.deposit(
        {
            args: {
                incognito_address: incognitoAddress,
                tx,
                signature: (ethers.utils.joinSignature(signature).slice(0, -2) + '0' + signature.recoveryParam.toString()).slice(2)
            },
            gas: "300000000000000",
            amount: "1000000000000000000000"
        },
    );

})();

function toHexString(byteArray) {
    return Array.from(byteArray, function(byte) {
        return ('0' + (byte & 0xFF).toString(16)).slice(-2);
    }).join('')
}