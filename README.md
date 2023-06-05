# sui-sponsor
Sui sponsor-as-a-serrice

## Create a local key for testing

```bash
# 1. Create a new local key for testing
sui keytool generate ed25519 word24

# 2. Move the above file to the keys folder

# 3. Get the base64 value of the private key and use it as `SPONSOR_PRIV_KEY` env variable
sui keytool  base64-to-hex <content_from_the_key_file_created_above>
```

## Create a local .env file

```
ENV=development
RUST_LOG=debug
PORT=4000
CORS_ORIGIN=*
SPONSOR_PRIV_KEY==
SUI_RPC=https://fullnode.devnet.sui.io:443
FIREBASE_API_KEY=
REDIS_HOST=127.0.0.1
REDIS_PORT=6379
REDIS_PASSWORD=
// Max number of coin objects in the pool
MAX_POOL_CAPACITY=
// Minimum number of coins that must always exist in the pool
MIN_POOL_COUNT=
// The balance of each coin that is created and added to the pool
COIN_BALANCE=
RABBITMQ_URI=amqp://user:password@localhost:5672
// Miliseconds to wait before the nacked message is set back to the RabbitMQ process to be retried
RETRY_TTL=1000
```

## Coin Manager
The role of CoinManager is to merge small coins into a single one and the split those into smaller ones. Those smaller coins will be added into the Gas Pool and later consumer by the GasPool service. In essence, this service will make sure that the GasPool has always enough Gas Coins and that the Sponsor account does not have too many dust Gas Coins. More specicifaclly, Gas Coins are used in sponsored transactions and thus their balance is getting low over time. At some point each such Gas coin will be so small that it cannot be used in any sponsored transaction. CoinManager will make sure to clear up those dust coins and recreate big enough coins which are added back to the Gas Pool.

The rebalance process is as follows:
- Merge all object that are not currently in the Gas Pool into a single Coin. The single coins is called master coin and it's the largest (in balance) coin that Sponsor account holds.
- Split the above master coin into enough new Coin objects to fill the Gas Pool. The number of coins to be created is `MAX_POOL_CAPACITY - CURRENT_POOL_COUNT`.

We use a Programmable Transaction Block to run these two transaction in a single Block Transaction. The Coin Manager will us the first coin as the master coin as explained above. It will also use the second largest coins as the one that will be used to 
