# sybil

Sybil is a canister that provides API to get assets data.

## Deploy local

```sh
make all
```

## Deploy prod

```sh
EXCHANGE_RATE_CANISTER_ID="uf6dk-hyaaa-aaaaq-qaaaq-cai"

dfx build sybil --network ic && gzip -f -1 ./.dfx/ic/canisters/sybil/sybil.wasm
dfx canister install --wasm ./.dfx/ic/canisters/sybil/sybil.wasm.gz --argument "(record {exchange_rate_canister=principal \"${EXCHANGE_RATE_CANISTER_ID}\"; mock=false; key_name=\"key_1\"; balances_cfg=record {rpc=\"{RPC_URL}\"; fee_per_byte=1:nat; chain_id=1:nat; erc20_contract=\"0x6b175474e89094c44da98b954eedeac495271d0f\"}})" --network ic sybil
```

## Upgrade prod

```sh
dfx build sybil --network ic && gzip -f -1 ./.dfx/ic/canisters/sybil/sybil.wasm &&
dfx canister install --mode upgrade --wasm ./.dfx/ic/canisters/sybil/sybil.wasm.gz --network ic sybil
```

## Upgrade local

```sh
make local_upgrade
```

## Enviroment

```sh
CALLER="0x6696eD42dFBe875E60779b8163fDCc39B088222A" &&
SIWE_MSG="localhost:4361 wants you to sign in with your Ethereum account:
0x6696eD42dFBe875E60779b8163fDCc39B088222A

Sign in with Ethereum.

URI: http://localhost:4361
Version: 1
Chain ID: 324
Nonce: NUY87tYWuZwkxrTZM
Issued At: 2023-11-03T11:40:39.690Z" &&
SIWE_SIG="31f8f8ea2104062e242dc13b9729c75b866e1ab1635c69404a1e7438221ff23849ea6a82e2544d28b4a16075f27fd3db6569e8664191af501572ad342e616c0300" 
TX_HASH="{Enter tx hash here, where you sent some tokens to the sybil address}" 
```

## Usage

```sh
dfx canister call sybil add_to_whitelist "(\"${CALLER}\")" 
dfx canister call sybil eth_address
dfx canister call sybil deposit "(\"${TX_HASH}\", \"${SIWE_MSG}\", \"${SIWE_SIG}\")"
dfx canister call sybil get_balance "(\"${CALLER}\")"
dfx canister call sybil add_to_balances_whitelist "(vec {})"
# create custom http feed
dfx canister call sybil create_custom_feed "(record {id=\"BTC/USDT\"; feed_type=variant {Custom}; update_freq=3600:nat; decimals=opt 6; sources=vec {variant { HttpSource = record {uri=\"https://api.pro.coinbase.com/products/{key1}/candles?granularity=60\"; api_keys = opt vec {record { title = \"key1\"; key = \"BTC-USDT\"}}; resolver=\"/0/1\"}}};msg=\"${SIWE_MSG}\"; sig=\"${SIWE_SIG}\"})"
dfx canister call sybil get_asset_data "(\"custom_BTC/USDT\", opt \"${SIWE_MSG}\", opt \"${SIWE_SIG}\")"
dfx canister call sybil remove_custom_feed "(\"custom_BTC/USDT\", \"${SIWE_MSG}\", \"${SIWE_SIG}\")"

# create custom getlogs feed 
# example of https://sepolia.etherscan.io/tx/0xb7c9735ec4c7b0996cb43a302d2209784cbe706fe0c9f50feda2c626fc6668ec#eventlog
dfx canister call sybil create_custom_feed "(record {id=\"get_logs_example\"; feed_type=variant {CustomString}; update_freq=3600:nat; decimals=null; sources=vec {variant { EvmEventLogsSource = record {rpc=\"https://endpoints.omniatech.io/v1/eth/sepolia/public\"; block_hash = opt \"0x31c066528f2b7800cb4d797b4ccb7c2f141f787b285ced26a3fdd1bafd66eb67\"; topic = opt \"0xdc9a6ce9bdf5d7327deb64beb9074cf0bc6e6c9ca2b318dae8b8ad4d38dd9344\"; address = opt \"0x67de6b66516E098EF945EAddE48C54fABfD3Dcf9\"; log_index = 0; event_log_field_name = \"dataFeedId\"; event_name = \"PriceFeedRequested\"; event_abi = \"[{\\\"type\\\":\\\"event\\\",\\\"name\\\":\\\"PriceFeedRequested\\\",\\\"inputs\\\":[{\\\"name\\\":\\\"dataFeedId\\\",\\\"type\\\":\\\"string\\\",\\\"indexed\\\":false,\\\"internalType\\\":\\\"string\\\"},{\\\"name\\\":\\\"callbackGasLimit\\\",\\\"type\\\":\\\"uint256\\\",\\\"indexed\\\":false,\\\"internalType\\\":\\\"uint256\\\"},{\\\"name\\\":\\\"requester\\\",\\\"type\\\":\\\"address\\\",\\\"indexed\\\":true,\\\"internalType\\\":\\\"address\\\"}],\\\"anonymous\\\":false}]\"}}};msg=\"${SIWE_MSG}\"; sig=\"${SIWE_SIG}\"})"
dfx canister call sybil get_asset_data "(\"custom_get_logs_example\", opt \"${SIWE_MSG}\", opt \"${SIWE_SIG}\")"
dfx canister call sybil remove_custom_feed "(\"custom_get_logs_example\", \"${SIWE_MSG}\", \"${SIWE_SIG}\")"

# create default feed (feeds come from xrc)
dfx canister call sybil create_default_feed "(record {id=\"ETH/USD\"; update_freq=360:nat; decimals=6:nat})"
dfx canister call sybil get_asset_data "(\"ETH/USD\", opt \"${SIWE_MSG}\", opt \"${SIWE_SIG}\")"
dfx canister call sybil get_asset_data_with_proof "(\"ETH/USD\", opt \"${SIWE_MSG}\", opt \"${SIWE_SIG}\")"
dfx canister call sybil remove_default_feed "(\"ETH/USD\")"

dfx canister call sybil get_multiple_assets_data "(vec { \"ETH/USD\"; \"custom_get_logs_example\" }, opt \"${SIWE_MSG}\", opt \"${SIWE_SIG}\")"
dfx canister call sybil get_multiple_assets_data_with_proof "(vec { \"ETH/USD\"; \"custom_get_logs_example\" }, opt \"${SIWE_MSG}\", opt \"${SIWE_SIG}\")"

dfx canister call sybil update_cfg "(record {evm_rpc_canister = opt \"aovwi-4maaa-aaaaa-qaagq-cai\"})"

dfx canister call sybil get_feeds "(null, null, opt \"${SIWE_MSG}\", opt \"${SIWE_SIG}\")"
dfx canister call sybil withdraw "(1:nat, \"${CALLER}\", \"${SIWE_MSG}\", \"${SIWE_SIG}\")"
dfx canister call sybil withdraw_fees "(\"${CALLER}\")"
```

## Test Enviroment Set Up

```sh
cd tests
npm install pm2@latest -g
npm init -y
npm install --save-dev hardhat
cd -
```

## Test Set Up

```sh
# Run an ICP replica
dfx start --clean --background

cd tests

# Run a Hardhat node
pm2 start 'npx hardhat node' # to stop use: pm2 stop all
# Run a deploy erc20 mock
export ERC20_MOCK_ADDR=$(npx hardhat run scripts/deploy.js)

cd -

# Deploy Sybil
dfx deploy exchange_rate_canister
EXCHANGE_RATE_CANISTER_ID=$(dfx canister id exchange_rate_canister)
dfx canister create sybil && dfx build sybil && gzip -f -1 ./.dfx/local/canisters/sybil/sybil.wasm
dfx canister install --wasm ./.dfx/local/canisters/sybil/sybil.wasm.gz --argument "(record {exchange_rate_canister=principal\"${EXCHANGE_RATE_CANISTER_ID}\"; mock=true; key_name=\"dfx_test_key\"; balances_cfg=record {rpc=\"http://localhost:8545\"; fee_per_byte=1:nat; chain_id=31337:nat; erc20_contract=\"${ERC20_MOCK_ADDR}\"}})" sybil
export SYBIL_ADDRESS=$(dfx canister call sybil eth_address | grep -oE '0x[0-9a-fA-F]{40}')

cd tests

## send some tokens and eth to the sybil address
export TX_HASH=$(npx hardhat run scripts/transfer.js)

```
