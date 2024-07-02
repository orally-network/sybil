use std::{collections::HashMap, str::FromStr};

use base64::Engine;
use candid::Principal;
use ethers_core::k256::sha2::{Digest, Sha256};
use http::post;
use ic_cdk::update;

use serde_bytes::ByteBuf;
use stellar_strkey::ed25519::PublicKey;
use stellar_xdr::next::{
    AccountEntry, AccountId, Asset, DecoratedSignature, LedgerEntryData, LedgerKey,
    LedgerKeyAccount, Limits, Memo, MuxedAccount, Operation, OperationBody, PaymentOp,
    Preconditions, ReadXdr, Signature, SignatureHint, TransactionEnvelope, TransactionExt,
    TransactionResult, TransactionV1Envelope, Uint256, VecM, WriteXdr,
};
use types::{
    SchnorrAlgorithm, SchnorrKeyId, SchnorrPublicKeyArgs, SchnorrPublicKeyResult,
    SignWithSchnorrArgs, SignWithSchnorrResult,
};

use crate::{clone_with_state, log};

mod http;
pub mod types;

#[update]
pub async fn test() -> Result<(), String> {
    _test().await.map_err(|e| format!("failed to test: {}", e))
}

#[inline]
pub async fn _test() -> Result<(), String> {
    let schnor_canister = Principal::from_text("dzh22-nuaaa-aaaaa-qaaoa-cai").unwrap();
    // let stellar_rpc = "https://horizon-testnet.stellar.org/".to_string();
    let stellar_rpc = "https://soroban-testnet.stellar.org:443".to_string();

    let args = (SchnorrPublicKeyArgs {
        canister_id: None,
        derivation_path: vec![],
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Ed25519,
            name: clone_with_state!(key_name),
        },
    },);

    let result: (SchnorrPublicKeyResult,) =
        ic_cdk::api::call::call(schnor_canister, "schnorr_public_key", args)
            .await
            .unwrap();

    // let staging = "GBEHNMD2NW7C3NQERWHJLXQQLLDFIEBO4GOHK675S7OCMOYKXUVT4W3T";

    // let test_pubkey = [
    //     59u8, 106, 39, 188, 206, 182, 164, 45, 98, 163, 168, 208, 42, 111, 13, 115, 101, 50, 21,
    //     119, 29, 226, 67, 166, 58, 192, 72, 161, 139, 89, 218, 41,
    // ];

    let pubkey: [u8; 32] = result.0.public_key.into_vec().try_into().unwrap();

    log!(
        "Pubkey: {:?}",
        PublicKey::from_payload(&pubkey).unwrap().to_string()
    );

    let account_id = AccountId(stellar_xdr::next::PublicKey::PublicKeyTypeEd25519(Uint256(
        pubkey, // test_pubkey,
    )));

    let mut account_entry = load_account(account_id, &stellar_rpc).await;

    // Account's sequence number must be incremented before sending a transaction
    account_entry.seq_num.0 += 1;
    let source_account = MuxedAccount::Ed25519(Uint256(pubkey));

    let tx_obj = stellar_xdr::next::Transaction {
        source_account: source_account.clone(),
        fee: 100,
        seq_num: account_entry.seq_num, // Sequence number of the account
        cond: Preconditions::None,
        memo: Memo::None,
        operations: vec![Operation {
            source_account: Some(source_account),
            // source_account: None,
            body: OperationBody::Payment(PaymentOp {
                destination: MuxedAccount::from_str(
                    "GBEHNMD2NW7C3NQERWHJLXQQLLDFIEBO4GOHK675S7OCMOYKXUVT4W3T",
                )
                .unwrap(),
                asset: Asset::Native,
                amount: 2,
            }),
        }]
        .try_into()
        .unwrap(),

        ext: TransactionExt::V0,
    };

    let network_passphrase = "Test SDF Network ; September 2015".to_string();

    let tagged_tx =
        stellar_xdr::next::TransactionSignaturePayloadTaggedTransaction::Tx(tx_obj.clone());
    let tx_sig = stellar_xdr::next::TransactionSignaturePayload {
        network_id: stellar_xdr::next::Hash(Sha256::digest(network_passphrase.as_str()).into()),
        tagged_transaction: tagged_tx,
    };

    let tx_hash: [u8; 32] =
        Sha256::digest(tx_sig.to_xdr(stellar_xdr::next::Limits::none()).unwrap()).into();

    let args = (SignWithSchnorrArgs {
        message: ByteBuf::from(tx_hash),
        derivation_path: vec![],
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Ed25519,
            name: clone_with_state!(key_name),
        },
    },);

    let result: (SignWithSchnorrResult,) =
        ic_cdk::api::call::call(schnor_canister, "sign_with_schnorr", args)
            .await
            .unwrap();

    let signature = Signature::try_from(result.0.signature.to_vec()).unwrap();
    let hint_val = &account_entry.account_id.to_xdr(Limits::none()).unwrap();
    let hint = SignatureHint::try_from(&hint_val[hint_val.len() - 4..]).unwrap();

    log!("Account id: {:?}", hint);
    log!("Hint: {:?}", hint);
    log!("Signature: {:?}", account_entry.account_id);

    let decorated_signature = DecoratedSignature { hint, signature };

    let signatures: VecM<DecoratedSignature, 20> = vec![decorated_signature].try_into().unwrap();

    let transaction_v1 = TransactionV1Envelope {
        tx: tx_obj,
        signatures,
    };

    let envelope = TransactionEnvelope::Tx(transaction_v1);

    let mut map: HashMap<String, serde_json::Value> = HashMap::new();

    map.insert(
        "transaction".to_string(),
        serde_json::Value::String(envelope.to_xdr_base64(Limits::none()).unwrap()),
    );

    let response = post::<serde_json::Value>(&stellar_rpc, "sendTransaction", map)
        .await
        .unwrap();

    if let Some(err) = response["result"].get("errorResultXdr") {
        let error_bytes = base64::engine::general_purpose::STANDARD
            .decode(err.as_str().unwrap())
            .unwrap();
        let tx_res = TransactionResult::from_xdr(&error_bytes, Limits::none()).unwrap();
        log!("Error: {:#?}", tx_res);
    }

    log!("Result: {:#?}", response);

    Ok(())
}

pub async fn load_account(account_id: AccountId, stellar_rpc: &str) -> AccountEntry {
    let mut map = HashMap::new();

    let key = LedgerKey::Account(LedgerKeyAccount { account_id });

    map.insert(
        "keys".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String(
            key.to_xdr_base64(Limits::none()).unwrap(),
        )]),
    );

    let response = post::<serde_json::Value>(stellar_rpc, "getLedgerEntries", map)
        .await
        .unwrap();

    let ledger_account = LedgerEntryData::from_xdr_base64(
        response["result"]["entries"][0]["xdr"].as_str().unwrap(),
        Limits::none(),
    )
    .unwrap();

    let LedgerEntryData::Account(account_entry) = ledger_account else {
        panic!("Not an account!");
    };

    account_entry
}
