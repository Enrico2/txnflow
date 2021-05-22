use std::{env, io};

use futures::executor::block_on;

use txnflow::{run_txn_processor, TxnFlowError};

fn main() {
    env_logger::init();

    let mut args: Vec<String> = env::args().collect();

    // expect a single argument on top of the executable
    if args.len() != 2 {
        panic!("{}", TxnFlowError::InvalidArguments)
    }

    let filename = args.pop().expect("args must be non empty");

    if let Err(e) = block_on(run_txn_processor(filename, &mut io::stdout())) {
        panic!("{}", e);
    }
}
