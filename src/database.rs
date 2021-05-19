use std::collections::HashMap;
use std::io;

use async_trait::async_trait;

use crate::accounts::UserAccount;
use crate::{Result, Txn, TxnFlowError, TxnType};

#[async_trait]
pub(crate) trait TxnStore {
    // TODO(ran) FIXME: use Result object? Or is Future enough.
    async fn get_txn(&self, id: u32) -> Option<&StoredTxn>;
    async fn get_txn_mut(&mut self, id: u32) -> Option<&mut StoredTxn>;

    // NB: In a prod system where this trait can be implemented with network calls,
    // the return type would be a Result
    async fn store_txn(&mut self, txn: &Txn) -> ();
}

#[async_trait]
pub(crate) trait AccountsStore {
    async fn get_or_create_account(&mut self, id: u16) -> &mut UserAccount;
}

pub(crate) struct MemTxnStore {
    txns: HashMap<u32, StoredTxn>,
}

pub(crate) enum StoredTxnType {
    Deposit,
    Withdrawal,
}

pub(crate) struct StoredTxn {
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
    async fn store_txn(&mut self, txn: &Txn) -> () {
        match &txn.kind {
            TxnType::Deposit => {
                self.txns.insert(
                    txn.id,
                    StoredTxn {
                        kind: StoredTxnType::Deposit,
                        amount: txn
                            .amount
                            .expect("`deposit` transaction must contain `amount`"),
                        disputed: false,
                    },
                );
            }
            TxnType::Withdrawal => {
                self.txns.insert(
                    txn.id,
                    StoredTxn {
                        kind: StoredTxnType::Withdrawal,
                        amount: txn
                            .amount
                            .expect("`withdrawal` transaction must contain `amount`"),
                        disputed: false,
                    },
                );
            }
            _ => (),
        }
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

// TODO(ran) FIXME: test this.
