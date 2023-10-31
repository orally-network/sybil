all: local_deploy_exchange_rate_canister local_deploy_sybil

local_deploy_exchange_rate_canister:
	dfx deploy exchange_rate_canister

local_deploy_sybil: local_deploy_exchange_rate_canister
	$(eval EXCHANGE_RATE_CANISTER_ID := $(shell dfx canister id exchange_rate_canister))
	dfx canister create sybil && dfx build sybil 
	gzip -f -1 ./.dfx/local/canisters/sybil/sybil.wasm
	dfx canister install --wasm ./.dfx/local/canisters/sybil/sybil.wasm.gz --argument \
		"(record {exchange_rate_canister=principal\"${EXCHANGE_RATE_CANISTER_ID}\"; mock=true; key_name=\"dfx_test_key\"; balances_cfg=record {rpc=\"https://sepolia.infura.io/v3/d20be327500c45819a1a3b850daec0e2\"; fee_per_byte=1:nat; chain_id=11155111:nat; erc20_contract=\"0xe37d61a6dc5573bdd4c9d2658bbfde5a58f9cea9\"}})" sybil

local_upgrade: local_upgreade_exchange_rate_canister local_upgrade_sybil

local_upgrade_exchange_rate_canister:
	dfx canister install --mode upgrade --wasm ./exchange_rate_canister.wasm exchange_rate_canister 

local_upgrade_sybil:
	dfx build sybil 
	gzip -f -1 ./.dfx/local/canisters/sybil/sybil.wasm
	dfx canister install --mode upgrade --wasm ./.dfx/local/canisters/sybil/sybil.wasm.gz sybil

clean:
	cargo clean