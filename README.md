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
```
