use crate::database::{AccountsStore, TxnStore};
use crate::{Result, Txn, TxnFlowError, TxnType};

pub(crate) struct TxnManager<'a> {
    pub(crate) txn_store: &'a mut dyn TxnStore,
    pub(crate) account_store: &'a mut dyn AccountsStore,
}

impl<'a> TxnManager<'a> {
    pub async fn process(&mut self, txn: Txn) -> Result<()> {
        // Store the transaction and fetch the user account in parallel
        let (store_txn_result, account) = futures::future::join(
            self.txn_store.store_txn(&txn),
            self.account_store.get_or_create_account(txn.client),
        )
        .await;

        // don't process transaction that failed storage.
        let _ = store_txn_result?;

        match &txn.kind {
            TxnType::Deposit => account.process_deposit(txn),

            TxnType::Withdrawal => account.process_withdrawal(txn),

            TxnType::Dispute => match self.txn_store.get_txn_mut(txn.id).await {
                // ensure the referenced txn exists and that it belongs to the client.
                Some(ref_txn) if ref_txn.client == txn.client => {
                    if !ref_txn.disputed {
                        let result = account.process_dispute(ref_txn);
                        if result.is_ok() {
                            ref_txn.disputed = true;
                        }
                        result
                    } else {
                        Err(TxnFlowError::TransactionAlreadyDisputed)
                    }
                }
                _ => Err(TxnFlowError::InvalidTransactionRef(txn.id)),
            },

            TxnType::Resolve => match self.txn_store.get_txn_mut(txn.id).await {
                // ensure the referenced txn exists and that it belongs to the client.
                Some(ref_txn) if ref_txn.client == txn.client => {
                    if ref_txn.disputed {
                        let result = account.process_resolve(ref_txn);
                        if result.is_ok() {
                            ref_txn.disputed = false
                        }
                        result
                    } else {
                        Err(TxnFlowError::TransactionNotDisputed)
                    }
                }
                _ => Err(TxnFlowError::InvalidTransactionRef(txn.id)),
            },

            TxnType::Chargeback => match self.txn_store.get_txn(txn.id).await {
                Some(ref_txn) if ref_txn.client == txn.client => {
                    if ref_txn.disputed {
                        account.process_chargeback(ref_txn)
                    } else {
                        Err(TxnFlowError::TransactionNotDisputed)
                    }
                }
                _ => Err(TxnFlowError::InvalidTransactionRef(txn.id)),
            },
        }
    }
}
