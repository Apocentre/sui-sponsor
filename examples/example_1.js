import fetch from 'node-fetch';
import {
  Ed25519Keypair,
  JsonRpcProvider,
  RawSigner,
  TransactionBlock,
  fromB64,
} from '@mysten/sui.js';
import config from './.config.json'  assert {type: "json"};

const importPrivateKey = (base64Key) => {
  const raw = fromB64(base64Key)
  const keypair = Ed25519Keypair.fromSecretKey(raw.slice(1))
  return keypair
}

const main = async () => {
  // Create a simple transaction block
  const keypair = importPrivateKey(config.secretKey);
  const provider = new JsonRpcProvider();
  const signer = new RawSigner(keypair, provider);
  const tx = new TransactionBlock();
  const [coin] = tx.splitCoins(tx.gas, [tx.pure(1000)]);
  tx.transferObjects([coin], tx.pure(keypair.getPublicKey().toSuiAddress()));
  tx.setSender(await signer.getAddress())

  // Request GasData

  const tx_data_bytes = await tx.build({provider, onlyTransactionKind: false});
  // convert the byte array to a base64 encoded string
  const tx_data = btoa(
    tx_data_bytes.reduce((data, byte) => data + String.fromCharCode(byte), '')
  );

  const response = await fetch('http://127.0.0.1:4000/gas/new', {
    method: 'post',
    body: JSON.stringify({tx_data}),
    headers: {'Content-Type': 'application/json'}
  });
  
  const data = await response.json();
  console.log(">>>>>>>>", data)

  // Sign the final transaction including the GasData

  // Requet the sponsor to transmit the transaction
}

main()
.then(() => console.log("Success"))
.catch((error) => console.log("Error: ", error))
