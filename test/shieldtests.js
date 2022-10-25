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

    const contractId = "inc.prv.testnet";

    const beacon_list = [
        // testnet
        // "7ef17C60cAa1c5C43d2Af62726c8f7c14000AB02",
        // "Fe30C03E5Db66236a82b0Dd204E811444Ba7761E",
        // "a357789d21e217FE3a30c7320A867B8B47C793A4",
        // "cc817963abe49569Ac58f1BC047B38cDA95832a1",

        // mainnet
        "e1fe6bdb4FB5f80801D242480c5467c1de94719c",
        "D57Dc32f9753a20Af166F9Dc48dE22355A9F7c83",
        "44b39171D742C2CdFdA0EBb6226f8584CAfc57FC",
        "4c8b59d24f07192B9095DA1EAE9af5c890413A71",
        "6d678311c5DAf5F8c8c48223C7Aea2A49D8d8B12",
        "93114859F53F98dC2a1FA6be9340Ce3B1D74722B",
        "0c7d24b75bEc5E94924e8e5d6c793609e48e7FF6",
    ];

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
    await contract.new(
        {
            args: {
                beacons: beacon_list,
                height: 0,
            },
            gas: "300000000000000",
            amount: "0"
        },
    );

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