use std::{env, io};

use futures::executor::block_on;

use txnflow::{run_txn_processor, TxnFlowError};

fn main() {
    let mut args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("{}", TxnFlowError::InvalidArguments)
    }

    let filename = args.pop().expect("Validated length is 2");

    if let Err(e) = block_on(run_txn_processor(filename, &mut io::stdout())) {
        eprintln!("{}", e);
    }
}

// TODO(ran) FIXME: 4 precision floats: https://stackoverflow.com/questions/39383809/how-to-transform-fields-during-serialization-using-serde
// TODO(ran) FIXME: log ignore and failure cases
// TODO(ran) FIXME: add docstrings
// TODO(ran) FIXME: Run clippy
// TODO(ran) FIXME: get more test coverage
// TODO(ran) FIXME: run coverage in github?
