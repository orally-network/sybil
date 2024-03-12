all: local_deploy_sybil 

local_deploy_xrc:
	dfx deploy xrc


local_deploy_sybil: local_deploy_xrc
	$(eval RPC_URL?=https://ethereum-goerli.publicnode.com)
	$(eval XRC_ID := $(shell dfx canister id xrc))
	$(eval FALLBACK_XRC := $(shell dfx canister id xrc))

	dfx deploy evm_rpc --argument '(record { nodesInSubnet = 28 })'

	$(eval EVM_RPC_CANISTER := $(shell dfx canister id evm_rpc))

	dfx canister create sybil && dfx build sybil 
	gzip -f -1 ./.dfx/local/canisters/sybil/sybil.wasm
	dfx canister install --wasm ./.dfx/local/canisters/sybil/sybil.wasm.gz --argument \
		"(record {exchange_rate_canister=principal\"${XRC_ID}\"; fallback_xrc=principal\"${XRC_ID}\"; evm_rpc_canister=principal\"${EVM_RPC_CANISTER}\"; rpc_wrapper=\"https://rpc.orally.network/?rpc=\"; mock=true; key_name=\"dfx_test_key\"; \
			balances_cfg=record {rpc=\"${RPC_URL}\"; fee_per_byte=0:nat; chain_id=5:nat; erc20_contract=\"0xfad6367E97217cC51b4cd838Cc086831f81d38C2\"; \
			whitelist = vec {};
			}})" sybil

local_upgrade: local_upgrade_xrc local_upgrade_sybil

local_upgrade_xrc:
	dfx canister install --mode upgrade --wasm ./xrc.wasm.gz xrc 

local_upgrade_sybil:
	dfx build sybil 
	gzip -f -1 ./.dfx/local/canisters/sybil/sybil.wasm
	dfx canister install --mode upgrade --wasm ./.dfx/local/canisters/sybil/sybil.wasm.gz sybil


ic_upgrade: ic_upgrade_xrc ic_upgrade_sybil 

ic_upgrade_xrc:
	dfx canister install --mode upgrade --wasm ./xrc.wasm.gz --network ic xrc 


ic_upgrade_sybil:
	dfx build sybil --network ic && gzip -f -1 ./.dfx/ic/canisters/sybil/sybil.wasm
	dfx canister install --mode upgrade --wasm ./.dfx/ic/canisters/sybil/sybil.wasm.gz --network ic sybil

clean:
	cargo clean
