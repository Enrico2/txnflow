use std::io;
use std::path::Path;

use csv::Trim;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use thiserror::Error;

pub use accounts::OutputAccount;

use crate::database::{MemAccountsStore, MemTxnStore};
use crate::transactions::TxnManager;

mod accounts;
mod database;
mod transactions;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TxnType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Txn {
    #[serde(alias = "tx")]
    pub(crate) id: u32,

    #[serde(alias = "type")]
    pub(crate) kind: TxnType,

    pub(crate) client: u16,

    pub(crate) amount: Option<f32>,
}

#[derive(Error, Debug)]
pub enum TxnFlowError {
    #[error("Insufficient funds for a withdrawal")]
    InsufficientFunds,

    #[error("`deposit` transaction must contain `amount`")]
    InvalidDepositTransaction,

    #[error("`withdrawal` transaction must contain `amount`")]
    InvalidWithdrawalTransaction,

    #[error("Transaction not found: {0}")]
    InvalidTransactionRef(u32),

    #[error("Referenced a transaction that is not under dispute")]
    TransactionNotDisputed,

    #[error("Disputing a transaction that is already under dispute")]
    TransactionAlreadyDisputed,

    #[error("Attempted transaction on a locked account")]
    LockedAccount,

    #[error("This program must be called with a single argument referring to a csv file")]
    InvalidArguments,

    #[error("Failed reading {0}")]
    IOException(String),

    #[error("Failed deserializing record in csv")]
    DeserializationException,

    #[error("Failed serializing record to csv")]
    SerializationException,
}

type Result<T> = std::result::Result<T, TxnFlowError>;

pub async fn run_txn_processor<W: io::Write>(filename: String, writer: &mut W) -> Result<()> {
    // Initialize stores and manager
    let mut txn_store = MemTxnStore::new();
    let mut account_store = MemAccountsStore::new();

    let mut txn_manager = TxnManager {
        txn_store: &mut txn_store,
        account_store: &mut account_store,
    };

    // Initialize csv reader
    let mut reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .from_path(Path::new(&filename))
        .map_err(|_| TxnFlowError::IOException(filename))?;

    // NB: Using unbounded here for convenience but really the buffer would be bounded
    // based on the system's throughput and capacity to handle a certain amount of in flight
    // transactions.
    let (mut txn_sender, mut txn_receiver) = mpsc::unbounded::<Txn>();

    for record in reader.deserialize() {
        // Should this be a transient error or not?
        // chose to fail completely but in a prod system, will only fail the request.
        let txn: Txn = record.map_err(|_| TxnFlowError::DeserializationException)?;

        if let Err(e) = txn_sender.send(txn).await {
            log::error!("Failed sending transaction to channel: {}", e)
        }
    }

    // closes the sender and causes the receiver to get a None once it's done getting all messages.
    drop(txn_sender);

    // Asynchronously process the transactions channel
    let runner = async move {
        while let Some(txn) = txn_receiver.next().await {
            if let Err(e) = txn_manager.process(txn).await {
                log_error(e)
            }
        }
    };

    // Await the async process
    runner.await;

    // Dump the account store to the given writer
    account_store.dump_to_csv(writer)?;

    Ok(())
}

fn log_error(e: TxnFlowError) {
    match e {
        TxnFlowError::InsufficientFunds
        | TxnFlowError::InvalidWithdrawalTransaction
        | TxnFlowError::InvalidDepositTransaction => log::error!("{}", e),
        TxnFlowError::LockedAccount => log::error!("{}", e),
        _ => log::warn!("{}", e),
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use csv::Trim;
    use serde::de::DeserializeOwned;

    use crate::accounts::OutputAccount;
    use crate::{run_txn_processor, Result};

    #[tokio::test]
    async fn test_main() {
        let mut buf = vec![];
        run_txn_processor(String::from("tests/test_data.csv"), &mut buf)
            .await
            .unwrap();

        let expected_contents = include_str!("../tests/expected_result.csv");

        let mut actual: Vec<OutputAccount> = read_csv(buf.as_slice()).unwrap();
        let mut expected: Vec<OutputAccount> = read_csv(expected_contents.as_bytes()).unwrap();

        actual.sort_by(|a, b| a.client.cmp(&b.client));
        expected.sort_by(|a, b| a.client.cmp(&b.client));

        assert_eq!(actual, expected)
    }

    fn read_csv<T: DeserializeOwned, R: io::Read>(reader: R) -> Result<Vec<T>> {
        let mut reader = csv::ReaderBuilder::new()
            .trim(Trim::All)
            .from_reader(reader);

        let mut result: Vec<T> = vec![];
        for record in reader.deserialize() {
            result.push(record.unwrap());
        }
        Ok(result)
    }
}
