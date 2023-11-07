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
dfx build sybil --network ic && gzip -f -1 ./.dfx/ic/canisters/sybil/sybil.wasm
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
SIWE_SIG="31f8f8ea2104062e242dc13b9729c75b866e1ab1635c69404a1e7438221ff23849ea6a82e2544d28b4a16075f27fd3db6569e8664191af501572ad342e616c0300" &&
TX_HASH="{Enter tx hash here, where you sent some tokens to the sybil address}" 
```

## Usage

```sh
dfx canister call sybil add_to_whitelist "(\"${CALLER}\")"
dfx canister call sybil eth_address
dfx canister call sybil deposit "(\"${TX_HASH}\", \"${SIWE_MSG}\", \"${SIWE_SIG}\")"
dfx canister call sybil get_balance "(\"${CALLER}\")"
dfx canister call sybil create_custom_pair "(record {pair_id=\"QUI/USD\"; update_freq=360:nat; decimals=6:nat; sources=vec {record {uri=\"https://aws.qui0scit.dev/\"; resolver=\"/rate\"; expected_bytes=1048576}};msg=\"${SIWE_MSG}\"; sig=\"${SIWE_SIG}\"})"
dfx canister call sybil get_asset_data "(\"QUI/USD\")"
dfx canister call sybil get_asset_data_with_proof "(\"QUI/USD\")"
dfx canister call sybil create_default_pair "(record {pair_id=\"ETH/USD\"; update_freq=360:nat; decimals=6:nat})"
dfx canister call sybil get_asset_data "(\"ETH/USD\")"
dfx canister call sybil get_asset_data_with_proof "(\"ETH/USD\")"
dfx canister call sybil get_pairs
dfx canister call sybil withdraw "(1:nat, \"${CALLER}\", \"${SIWE_MSG}\", \"${SIWE_SIG}\")"
dfx canister call sybil withdraw_fees "(\"${CALLER}\")"
dfx canister call sybil create_data_fetcher "(record {update_freq=360:nat; sources=vec {record {uri=\"https://aws.qui0scit.dev/\"; resolver=\"/symbol\"; expected_bytes=1048576}}; msg=\"${SIWE_MSG}\"; sig=\"${SIWE_SIG}\"})"
dfx canister call sybil get_data "(1:nat)"
dfx canister call sybil get_data_fetchers "(\"${CALLER}\")"
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
