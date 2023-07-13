# sybil

Sybil is a canister that provides API to get assets data.

## Deploy local

```sh
dfx deploy exchange_rate_canister
ECHANGE_RATE_CANISTER_ID=$(dfx canister id exchange_rate_canister)

dfx canister create sybil && dfx build sybil && gzip -f -1 ./.dfx/local/canisters/sybil/sybil.wasm
dfx canister install --wasm ./.dfx/local/canisters/sybil/sybil.wasm.gz --argument "(record {exchange_rate_canister=principal\"${ECHANGE_RATE_CANISTER_ID}\"; mock=true; key_name=\"dfx_test_key\"; balances_cfg=record {rpc=\"https://sepolia.infura.io/v3/d20be327500c45819a1a3b850daec0e2\"; fee_per_byte=1:nat; chain_id=11155111:nat; erc20_contract=\"0xe37d61a6dc5573bdd4c9d2658bbfde5a58f9cea9\"}})" sybil
```

## Upgrade local

```sh
dfx build sybil && gzip -f -1 ./.dfx/local/canisters/sybil/sybil.wasm
dfx canister install --mode upgrade --wasm ./.dfx/local/canisters/sybil/sybil.wasm.gz sybil
```

## Enviroment

```sh
CALLER="0xE86C4A45C1Da21f8838a1ea26Fc852BD66489ce9"
SIWE_MSG="service.org wants you to sign in with your Ethereum account:
0xE86C4A45C1Da21f8838a1ea26Fc852BD66489ce9


URI: https://service.org/login
Version: 1
Chain ID: 11155111
Nonce: 00000000
Issued At: 2023-05-04T18:39:24Z"
SIWE_SIG="fa7b336d271b7ed539b6db3034d57be294ef889b42534fa95689afd0989ab6d27878c837a14ed1b4c3ab6b7052180ce87198934cb7712a81ea413fd8ebb29e8c1c"
TX_HASH=""
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
dfx canister call sybil get_all_pairs
dfx canister call sybil withdraw "(1:nat, \"${CALLER}\", \"${SIWE_MSG}\", \"${SIWE_SIG}\")"
dfx canister call sybil withdraw_fees "(\"${CALLER}\")"
dfx canister call sybil create_data_fetcher "(record {update_freq=360:nat; sources=vec {record {uri=\"https://aws.qui0scit.dev/\"; resolver=\"/symbol\"; expected_bytes=1048576}}; msg=\"${SIWE_MSG}\"; sig=\"${SIWE_SIG}\"})"
dfx canister call sybil get_data "(1:nat)"
dfx canister call sybil get_data_fetchers "(\"${CALLER}\")"
```
