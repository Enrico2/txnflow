use serde::{Deserialize, Serialize};

use crate::database::{StoredTxn, StoredTxnType};
use crate::Result;
use crate::{Txn, TxnFlowError};
use float_cmp::approx_eq;

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputAccount {
    pub client: u16,
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

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
        if !self.locked {
            txn.amount
                .ok_or(TxnFlowError::InvalidDepositTransaction)
                .map(|n| self.available += n)
        } else {
            Err(TxnFlowError::LockedAccount)
        }
    }

    pub(crate) fn process_withdrawal(&mut self, txn: Txn) -> Result<()> {
        if !self.locked {
            let amount = txn
                .amount
                .expect("`withdrawal` transaction must contain `amount`");

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
            match ref_txn.kind {
                StoredTxnType::Deposit => {
                    self.held -= ref_txn.amount;
                }
                StoredTxnType::Withdrawal => {
                    self.held -= ref_txn.amount;
                    self.available += ref_txn.amount;
                }
            }

            self.locked = true;
            Ok(())
        } else {
            Err(TxnFlowError::LockedAccount)
        }
    }
}

// TODO(ran) FIXME: test this.
