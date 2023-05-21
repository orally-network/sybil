# sybil

## Deploy
```sh
dfx deploy exchange_rate_canister
dfx deploy sybil
dfx canister call sybil set_expiration_time '(3600:nat)'
dfx canister call sybil set_siwe_signer_canister '("bkyz2-fmaaa-aaaaa-qaaaq-cai")'
dfx canister call sybil set_exchange_rate_canister '("bw4dl-smaaa-aaaaa-qaacq-cai")'
dfx canister call sybil set_treasurer_canister '("be2us-64aaa-aaaaa-qaabq-cai")'
dfx canister call sybil set_key_name '("dfx_test_key")'
dfx canister call sybil set_cost_per_execution '(1)'
```

## Usage
```sh 
# to create a custom pair
dfx canister call sybil create_custom_pair '(record {pair_id="QUI/USDT"; frequency=60:nat; uri="https://aws.qui0scit.dev/"; resolver="/rate"; amount=1:nat; msg="service.org wants you to sign in with your Ethereum account:
0xE86C4A45C1Da21f8838a1ea26Fc852BD66489ce9


URI: https://service.org/login
Version: 1
Chain ID: 11155111
Nonce: 00000000
Issued At: 2023-05-04T18:39:24Z"; sig="fa7b336d271b7ed539b6db3034d57be294ef889b42534fa95689afd0989ab6d27878c837a14ed1b4c3ab6b7052180ce87198934cb7712a81ea413fd8ebb29e8c1c"})'

# to get a custom pair data
dfx canister call sybil get_asset_data_with_proof '("QUI/USDT")'
``` 