all: local_deploy_sybil 

local_deploy_xrc:
	dfx deploy xrc

local_deploy_evm_rpc:
	dfx deploy evm_rpc --argument '(record { nodesInSubnet = 28 })'


local_deploy_sybil: local_deploy_xrc local_deploy_evm_rpc
	$(eval ADDRESS?=0x6696eD42dFBe875E60779b8163fDCc39B088222A)
	$(eval RPC_URL?=https://ethereum-sepolia-rpc.publicnode.com)
	$(eval MAINNET_RPC_URL?=https://eth.llamarpc.com)
	$(eval XRC_ID := $(shell dfx canister id xrc))
	$(eval TREASURE_ADDRESS := $(shell dfx canister id xrc))
	$(eval FALLBACK_XRC := $(shell dfx canister id xrc))
	$(eval EVM_RPC_CANISTER := $(shell dfx canister id evm_rpc))

	dfx canister create sybil && dfx build sybil 
	gzip -f -1 ./.dfx/local/canisters/sybil/sybil.wasm
	dfx canister install --wasm ./.dfx/local/canisters/sybil/sybil.wasm.gz --argument \
		"(record { \
			exchange_rate_canister=principal\"${XRC_ID}\"; \
			fallback_xrc=principal\"${XRC_ID}\"; \
			evm_rpc_canister=principal\"${EVM_RPC_CANISTER}\"; \
			rpc_wrapper=\"https://rpc.orally.network/?rpc=\";  \
			mock=true; \
			key_name=\"dfx_test_key\"; \
			balances_cfg=record { \
				rpc=\"depricated\"; \
				chain_id=11155111:nat; \
				erc20_contract=\"depricated\"; \
				allowed_chains = vec {record { \
					11155111:nat64; record { \
						erc20_contracts = vec { record { \
							erc20_contract = \"0xD6CdFF58Dd98528730549c6E5EEdF1be397A723f\"; \
							token_symbol = \"SHR\"; \
							decimals = 6:nat64; \
						}}; \
						coin_symbol = \"Eth\"; \
						rpc = record { url = \"${RPC_URL}\"; secret=null; config=record { num_of_blocks_for_get_dxr_data = 100:nat64 } }; \
					} \
					}; record { \
						1:nat64; record { \
							erc20_contracts = vec {}; \
							coin_symbol = \"Eth\"; \
							rpc = record { url = \"${MAINNET_RPC_URL}\"; secret=null; config=record { num_of_blocks_for_get_dxr_data = 100:nat64 } }; \
						} \
					}; \
				}; \
				treasure_address=\"${ADDRESS}\"; \
				fee_per_byte=1:nat; \
				base_fee=1:nat; \
				signature_fee=1:nat; \
				whitelist = vec {}; \
			}\
		})" sybil

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
