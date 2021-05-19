// use futures::channel::mpsc::{Receiver, Sender};
// use crate::model::Txn;
// use crate::database::TxnStore;
//
// struct ClientUpdate {
//     client: u16,
//     available_update: f32,
//     held_update: f32
// }
//
// struct StreamingTxns {
//     txn_store: dyn TxnStore,
//     txn_receiver: Receiver<Txn>,
//     account_update: Sender<ClientUpdate>
// }
//
// impl StreamingTxns {
//
// }
