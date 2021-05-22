use std::collections::HashMap;
use std::io;

use async_trait::async_trait;

use crate::accounts::UserAccount;
use crate::{Result, Txn, TxnFlowError, TxnType};

#[async_trait]
pub(crate) trait TxnStore {
    async fn get_txn(&self, id: u32) -> Option<&StoredTxn>;
    async fn get_txn_mut(&mut self, id: u32) -> Option<&mut StoredTxn>;

    async fn store_txn(&mut self, txn: &Txn) -> Result<()>;
}

#[async_trait]
pub(crate) trait AccountsStore {
    async fn get_or_create_account(&mut self, id: u16) -> &mut UserAccount;
}

/////////////////////////////////////////////////
// In memory implementations of the Store traits
/////////////////////////////////////////////////
pub(crate) struct MemTxnStore {
    txns: HashMap<u32, StoredTxn>,
}

pub(crate) enum StoredTxnType {
    Deposit,
    Withdrawal,
}

pub(crate) struct StoredTxn {
    pub(crate) client: u16,
    pub(crate) kind: StoredTxnType,
    pub(crate) amount: f32,
    pub(crate) disputed: bool,
}

impl MemTxnStore {
    pub(crate) fn new() -> MemTxnStore {
        MemTxnStore {
            txns: HashMap::new(),
        }
    }
}

#[async_trait]
impl TxnStore for MemTxnStore {
    async fn get_txn(&self, id: u32) -> Option<&StoredTxn> {
        self.txns.get(&id)
    }

    async fn get_txn_mut(&mut self, id: u32) -> Option<&mut StoredTxn> {
        self.txns.get_mut(&id)
    }

    // This is called once per transaction and they're assumed unique.
    async fn store_txn(&mut self, txn: &Txn) -> Result<()> {
        match &txn.kind {
            TxnType::Deposit => {
                self.txns.insert(
                    txn.id,
                    StoredTxn {
                        client: txn.client,
                        kind: StoredTxnType::Deposit,
                        amount: txn.amount.ok_or(TxnFlowError::InvalidDepositTransaction)?,
                        disputed: false,
                    },
                );
            }
            TxnType::Withdrawal => {
                self.txns.insert(
                    txn.id,
                    StoredTxn {
                        client: txn.client,
                        kind: StoredTxnType::Withdrawal,
                        amount: txn
                            .amount
                            .ok_or(TxnFlowError::InvalidWithdrawalTransaction)?,
                        disputed: false,
                    },
                );
            }
            _ => (),
        };
        Ok(())
    }
}

pub(crate) struct MemAccountsStore {
    accounts: HashMap<u16, UserAccount>,
}

impl MemAccountsStore {
    pub fn new() -> MemAccountsStore {
        MemAccountsStore {
            accounts: HashMap::new(),
        }
    }

    pub(crate) fn dump_to_csv<W: io::Write>(&self, w: W) -> Result<()> {
        let mut writer = csv::Writer::from_writer(w);
        for (client, acct) in self.accounts.iter() {
            let write = writer
                .serialize(acct.as_output_account(*client))
                .map_err(|_| TxnFlowError::SerializationException);

            if write.is_err() {
                return write;
            }
        }
        writer
            .flush()
            .map_err(|_| TxnFlowError::SerializationException)?;
        Ok(())
    }
}

#[async_trait]
impl AccountsStore for MemAccountsStore {
    async fn get_or_create_account(&mut self, id: u16) -> &mut UserAccount {
        self.accounts.entry(id).or_insert_with(UserAccount::new)
    }
}
