use std::str::FromStr;

use ic_cdk::api::management_canister::ecdsa::sign_with_ecdsa;
use ic_cdk::{
    api::management_canister::ecdsa::{EcdsaCurve, EcdsaKeyId, SignWithEcdsaArgument},
    export::{
        candid::CandidType,
        serde::{Deserialize, Serialize},
    },
};
use ic_web3::{
    ethabi::Token,
    signing::{hash_message, keccak256},
    types::H160,
};

use anyhow::{anyhow, Result};

use crate::{
    utils::{
        encoding::encode_packed,
        signature::{eth_address, get_eth_v},
    },
    STATE,
};

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct RateDataLight {
    pub symbol: String,
    pub rate: u64,
    pub timestamp: u64,
}

impl RateDataLight {
    fn encode_packed(&self) -> Vec<u8> {
        let raw_data = vec![
            Token::String(self.symbol.clone()),
            Token::Uint(self.rate.into()),
            Token::Uint(self.timestamp.into()),
        ];

        encode_packed(&raw_data).expect("tokens should be valid")
    }

    pub async fn sign(&self) -> Result<String> {
        let sign_data = hash_message(keccak256(&self.encode_packed())).0.to_vec();

        let key_name = STATE.with(|state| state.borrow().key_name.clone());

        let call_args = SignWithEcdsaArgument {
            message_hash: sign_data.clone(),
            derivation_path: vec![ic_cdk::id().as_slice().to_vec()],
            key_id: EcdsaKeyId {
                curve: EcdsaCurve::Secp256k1,
                name: key_name,
            },
        };

        let (signature,) = sign_with_ecdsa(call_args)
            .await
            .map_err(|(code, msg)| anyhow!("{:?}: {}", code, msg))?;

        let mut signature = signature.signature;

        let pub_key = eth_address().await.map_err(|msg| anyhow!(msg))?;

        let v = get_eth_v(&signature, &sign_data, &H160::from_str(&pub_key)?)?;

        signature.push(v);

        Ok(hex::encode(signature))
    }
}

#[derive(Clone, Debug, Default, CandidType, Serialize, Deserialize)]
pub struct CustomPairData {
    pub data: RateDataLight,
    pub signature: String,
}

impl CustomPairData {
    pub async fn from_rate(rate: RateDataLight) -> Result<Self> {
        Ok(Self {
            data: rate.clone(),
            signature: rate.sign().await?,
        })
    }
}
