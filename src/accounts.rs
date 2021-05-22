use float_cmp::approx_eq;
use serde::{Deserialize, Serialize, Serializer};

use crate::database::{StoredTxn, StoredTxnType};
use crate::{Result, TxnType};
use crate::{Txn, TxnFlowError};

fn f32_serialize<S>(x: &f32, s: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_f32(f32::trunc(x * 10000.0) / 10000.0)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputAccount {
    pub client: u16,

    #[serde(serialize_with = "f32_serialize")]
    available: f32,

    #[serde(serialize_with = "f32_serialize")]
    held: f32,

    #[serde(serialize_with = "f32_serialize")]
    total: f32,
    locked: bool,
}

// NB: Technically this is only used by tests, except it's used both by integration tests and unit tests.
// for convenience this is part of this crate but normally I might put it in a separate utility crate
// to be used only by tests as a dev dependency.
impl PartialEq for OutputAccount {
    fn eq(&self, other: &Self) -> bool {
        self.client == other.client
            && self.locked == other.locked
            && approx_eq!(f32, self.available, other.available, ulps = 4)
            && approx_eq!(f32, self.held, other.held, ulps = 4)
            && approx_eq!(f32, self.total, other.total, ulps = 4)
    }
}

impl Eq for OutputAccount {}

#[derive(Debug)]
pub(crate) struct UserAccount {
    available: f32,
    held: f32,
    locked: bool,
}

impl UserAccount {
    pub(crate) fn as_output_account(&self, client: u16) -> OutputAccount {
        OutputAccount {
            client,
            available: self.available,
            held: self.held,
            total: self.available + self.held,
            locked: self.locked,
        }
    }
}

impl UserAccount {
    pub(crate) fn new() -> UserAccount {
        UserAccount {
            available: 0.0,
            held: 0.0,
            locked: false,
        }
    }

    pub(crate) fn process_deposit(&mut self, txn: Txn) -> Result<()> {
        assert_eq!(txn.kind, TxnType::Deposit);
        assert!(txn.amount.is_some());

        if !self.locked {
            self.available += txn.amount.unwrap();
            Ok(())
        } else {
            Err(TxnFlowError::LockedAccount)
        }
    }

    pub(crate) fn process_withdrawal(&mut self, txn: Txn) -> Result<()> {
        assert_eq!(txn.kind, TxnType::Withdrawal);
        assert!(txn.amount.is_some());

        if !self.locked {
            let amount = txn.amount.unwrap();

            if self.available >= amount {
                self.available -= amount;
                Ok(())
            } else {
                Err(TxnFlowError::InsufficientFunds)
            }
        } else {
            Err(TxnFlowError::LockedAccount)
        }
    }

    pub(crate) fn process_dispute(&mut self, ref_txn: &StoredTxn) -> Result<()> {
        if !self.locked {
            match ref_txn.kind {
                StoredTxnType::Deposit => {
                    self.available -= ref_txn.amount;
                    self.held += ref_txn.amount;
                }
                StoredTxnType::Withdrawal => {
                    self.held += ref_txn.amount;
                }
            }
            Ok(())
        } else {
            Err(TxnFlowError::LockedAccount)
        }
    }

    pub(crate) fn process_resolve(&mut self, ref_txn: &StoredTxn) -> Result<()> {
        if !self.locked {
            match ref_txn.kind {
                StoredTxnType::Deposit => {
                    self.available += ref_txn.amount;
                    self.held -= ref_txn.amount;
                }
                StoredTxnType::Withdrawal => {
                    self.held -= ref_txn.amount;
                }
            }
            Ok(())
        } else {
            Err(TxnFlowError::LockedAccount)
        }
    }

    pub(crate) fn process_chargeback(&mut self, ref_txn: &StoredTxn) -> Result<()> {
        if !self.locked {
            self.held -= ref_txn.amount;

            self.locked = true;
            Ok(())
        } else {
            Err(TxnFlowError::LockedAccount)
        }
    }
}
