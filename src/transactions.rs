use crate::database::{AccountsStore, TxnStore};
use crate::{Result, Txn, TxnFlowError, TxnType};

pub(crate) struct TxnManager<'a> {
    pub(crate) txn_store: &'a mut dyn TxnStore,
    pub(crate) account_store: &'a mut dyn AccountsStore,
}

impl<'a> TxnManager<'a> {
    pub async fn process(&mut self, txn: Txn) -> Result<()> {
        let txn_id = txn.id;
        let (_, account) = futures::future::join(
            self.txn_store.store_txn(&txn),
            self.account_store.get_or_create_account(txn.client),
        )
        .await;

        match &txn.kind {
            TxnType::Deposit => account.process_deposit(txn),
            TxnType::Withdrawal => account.process_withdrawal(txn),
            TxnType::Dispute => {
                if let Some(ref_txn) = self.txn_store.get_txn_mut(txn_id).await {
                    if !ref_txn.disputed {
                        ref_txn.disputed = true;
                        account.process_dispute(ref_txn)
                    } else {
                        Err(TxnFlowError::TransactionNotDisputed)
                    }
                } else {
                    Err(TxnFlowError::InvalidTransactionRef(txn_id))
                }
            }
            TxnType::Resolve => {
                if let Some(ref_txn) = self.txn_store.get_txn_mut(txn_id).await {
                    if !ref_txn.disputed {
                        let result = account.process_resolve(ref_txn);
                        if result.is_ok() {
                            ref_txn.disputed = false
                        }
                        result
                    } else {
                        Err(TxnFlowError::TransactionNotDisputed)
                    }
                } else {
                    Err(TxnFlowError::InvalidTransactionRef(txn_id))
                }
            }
            TxnType::Chargeback => {
                if let Some(ref_txn) = self.txn_store.get_txn(txn_id).await {
                    if !ref_txn.disputed {
                        account.process_chargeback(ref_txn)
                    } else {
                        Err(TxnFlowError::TransactionNotDisputed)
                    }
                } else {
                    Err(TxnFlowError::InvalidTransactionRef(txn_id))
                }
            }
        }
    }
}

// TODO(ran) FIXME: test this.
