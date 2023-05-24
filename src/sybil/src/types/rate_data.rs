use std::str::FromStr;

use candid::Principal;
use ic_cdk::api::management_canister::ecdsa::SignWithEcdsaResponse;
use ic_cdk::api::call::call_with_payment;
use ic_cdk::{
    api::management_canister::ecdsa::{EcdsaCurve, EcdsaKeyId, SignWithEcdsaArgument},
    export::{
        candid::CandidType,
        serde::{Deserialize, Serialize},
    },
};
use ic_web3::{ethabi::Token, signing::keccak256, types::H160};

use anyhow::{anyhow, Result};

use crate::{
    utils::{
        encoding::encode_packed,
        signature::{eth_address, get_eth_v},
    },
    STATE,
};

const ECDSA_SIGN_CYCLES: u64 = 23_000_000_000;

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct RateDataLight {
    pub symbol: String,
    pub rate: u64,
    pub decimals: u64,
    pub timestamp: u64,
    pub signature: Option<String>
}

impl RateDataLight {
    fn encode_packed(&self) -> Vec<u8> {
        let raw_data = vec![
            Token::String(self.symbol.clone()),
            Token::Uint(self.rate.into()),
            Token::Uint(self.decimals.into()),
            Token::Uint(self.timestamp.into()),
        ];

        encode_packed(&raw_data).expect("tokens should be valid")
    }

    pub async fn sign(&mut self) -> Result<String> {
        let sign_data = keccak256(&self.encode_packed()).to_vec();

        let key_name = STATE.with(|state| state.borrow().key_name.clone());

        let call_args = SignWithEcdsaArgument {
            message_hash: sign_data.clone(),
            derivation_path: vec![ic_cdk::id().as_slice().to_vec()],
            key_id: EcdsaKeyId {
                curve: EcdsaCurve::Secp256k1,
                name: key_name,
            },
        };

        let (signature,): (SignWithEcdsaResponse,) = call_with_payment(
            Principal::management_canister(),
            "sign_with_ecdsa",
            (call_args,),
            ECDSA_SIGN_CYCLES
        )
            .await
            .map_err(|(code, msg)| anyhow!("{:?}: {}", code, msg))?;

        let mut signature = signature.signature;

        let pub_key = eth_address().await.map_err(|msg| anyhow!(msg))?;

        let v = get_eth_v(&signature, &sign_data, &H160::from_str(&pub_key)?)?;

        signature.push(v);

        let signature = hex::encode(signature);

        self.signature = Some(signature.clone());

        Ok(signature)
    }
}
