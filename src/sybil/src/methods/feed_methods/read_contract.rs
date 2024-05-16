use ic_cdk::update;

use ic_web3_rs::contract::Contract;
use sybil_utils::cycles_count;

use crate::{
    clone_with_state, log,
    methods::{balances, custom_feeds::CustomFeedError},
    stringify_func_call,
    types::{
        balances::{BalanceError, Balances},
        cache::Cache,
        chains_rpc::ChainsRPC,
        feed_types::read_contract::{ReadContractMetadata, ReadContractResult, SolidityToken},
        state,
    },
    utils::{
        address, canister, convertion::convert_usd_to_eth, encoding::parse_tokens, siwe,
        time::in_seconds, web3,
    },
};

#[update]
pub async fn read_contract_with_proof(
    chain_id: u64,
    function_signature: String,
    contract_address: String,
    method: String,
    params: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<ReadContractResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _read_contract(
        chain_id,
        function_signature,
        contract_address,
        method,
        params,
        None,
        true,
        None,
    )
    .await
    .map_err(|e| format!("Failed to read contract: {e}"))
}

#[update]
pub async fn read_contract(
    chain_id: u64,
    function_signature: String,
    contract_address: String,
    method: String,
    params: String,
    msg: Option<String>,
    sig: Option<String>,
) -> Result<ReadContractResult, String> {
    let payer = if let (Some(msg), Some(sig)) = (msg, sig) {
        siwe::recover(&msg, &sig)
            .await
            .map_err(|err| err.to_string())?
    } else {
        ic_cdk::caller().to_string()
    };

    _read_contract(
        chain_id,
        function_signature,
        contract_address,
        method,
        params,
        None,
        false,
        None,
    )
    .await
    .map_err(|e| format!("Failed to read contract: {e}"))
}

#[inline]
#[cycles_count]
pub async fn _read_contract(
    chain_id: u64,
    function_signature: String,
    contract_addr: String,
    method: String,
    params: String,
    payer: Option<String>,
    with_signature: bool,
    cache_ttl: Option<u64>,
) -> Result<ReadContractResult, CustomFeedError> {
    let func_signature = stringify_func_call!(_read_contract(
        chain_id,
        function_signature,
        contract_addr,
        method,
        params,
        with_signature
    ));

    let func_body = async move {
        let base_fee = state::get_cfg().balances_cfg.base_fee;

        if let Some(ref payer) = payer {
            if !Balances::is_sufficient(payer, &base_fee)? {
                return Err(BalanceError::InsufficientBalance)?;
            };
        }

        let chain_rpc = ChainsRPC::get_first_chain_rpc(chain_id)?;

        let w3 = web3::instance(chain_rpc, clone_with_state!(evm_rpc_canister));

        let contract_address = address::to_h160(&contract_addr)?;

        let ethabi_contract = ethers_core::abi::parse_abi_str(&function_signature)
            .map_err(|err| CustomFeedError::FailedToParseABI(err.to_string()))?;

        let contract = Contract::new(w3.eth(), contract_address, ethabi_contract);

        let function = contract
            .abi()
            .function(&method)
            .map_err(|_| CustomFeedError::AbiDoesntContainMethod(method.clone()))?;

        let inputs = function
            .inputs
            .iter()
            .map(|p| p.kind.clone())
            .collect::<Vec<_>>();

        let tokens = parse_tokens(&inputs, params[1..params.len() - 1].to_string())?;

        let from = canister::eth_address().await?.to_string();

        let call_result = w3
            .get_call_result(
                &contract,
                &method,
                &tokens,
                address::to_h160(&from)?,
                Some(contract_address),
                None, // tx_hash.block_number,
            )
            .await?;

        let mut result = ReadContractResult {
            data: call_result.into_iter().map(SolidityToken::from).collect(),
            meta: ReadContractMetadata {
                chain_id,
                contract_address: contract_addr,
                method,
                params,
                timestamp: in_seconds(),
                fee: 0.into(),
            },
            signature: None,
        };

        if with_signature {
            result.sign().await?;
        }

        if let Some(payer) = payer {
            Balances::reduce_amount(&payer, &base_fee)?;
            Balances::add_amount(&canister::eth_address().await?, &base_fee)?;
        } else {
            let fee = convert_usd_to_eth(base_fee, balances::DECIMALS).await?;
            result.meta.fee = fee;
        }

        Ok(result)
    };

    Cache::with(func_signature, func_body, cache_ttl).await
}
