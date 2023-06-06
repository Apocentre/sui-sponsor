import fetch from 'node-fetch';
import {
  Connection,
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

const send = async () => {
  // Create a simple transaction block
  const keypair = importPrivateKey(config.secretKey);
  const provider = new JsonRpcProvider(new Connection({fullnode: config.rpcUrl}));
  const signer = new RawSigner(keypair, provider);
  const txb = new TransactionBlock();
  const [coin] = txb.splitCoins(txb.gas, [txb.pure(1000)]);
  txb.transferObjects([coin], txb.pure(keypair.getPublicKey().toSuiAddress()));
  txb.setSender(await signer.getAddress())

  // Request GasData

  const tx_data_bytes = await txb.build({provider, onlyTransactionKind: false});
  // convert the byte array to a base64 encoded string
  const tx_data = btoa(
    tx_data_bytes.reduce((data, byte) => data + String.fromCharCode(byte), '')
  );

  const response = await fetch('http://127.0.0.1:4000/tx/gas', {
    method: 'post',
    body: JSON.stringify({tx_data}),
    headers: {'Content-Type': 'application/json'}
  });
  
  const {
    gas_data: {
      payment,
      owner,
      price,
      budget
    },
  } = await response.json();

  // Sign the final transaction including the GasData
  txb.setGasBudget(budget)
  txb.setGasPayment(payment.map(p => ({
    objectId: p[0],
    version: p[1],
    digest: p[2],
  })))
  txb.setGasOwner(owner)
  txb.setGasPrice(price)

  let transactionBlock = await txb.build({provider, onlyTransactionKind: false});
  const signed_tx = await signer.signTransactionBlock({transactionBlock})

  // Request the sponsor to transmit the transaction
  const response_2 = await fetch('http://127.0.0.1:4000/tx/submit', {
    method: 'post',
    body: JSON.stringify({...signed_tx}),
    headers: {'Content-Type': 'application/json'}
  });

  console.log("Tx Result", JSON.stringify(await response_2.json(), null, 2))
}

const sleep = ts => new Promise((resolve) => setTimeout(resolve, ts))

const main = async () => {
  while(true) {
    const reqs = Array.from(new Array(50), () => send())
    await Promise.all(reqs)
    await sleep(5000)
  }
}

main()
.then(() => console.log("Success"))
.catch((error) => console.log("Error: ", error))
