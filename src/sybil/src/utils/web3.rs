use ic_web3_rs::{Web3, transports::ICHttp, types::{H256, TransactionReceipt}, Error as Web3ClientError};

use thiserror::Error;

use super::processors::transform_ctx_tx_with_logs;

#[derive(Error, Debug)]
pub enum Web3Error {
    #[error("client error: {0}")]
    ClientError(#[from] Web3ClientError),
}

#[inline(always)]
pub fn client(rpc: &str) -> Web3<ICHttp> {
    Web3::new(ICHttp::new(rpc, None)
        .expect("In this version of the ic web3 client this method will never fail, if it does, discart this version and use the one from the git repo of mine"))
}

pub async fn tx_receipt(w3: &Web3<ICHttp>, tx_hash: &H256) -> Result<Option<TransactionReceipt>, Web3Error> {
    Ok(w3
        .eth()
        .transaction_receipt(tx_hash.clone(), transform_ctx_tx_with_logs())
        .await?)
}
