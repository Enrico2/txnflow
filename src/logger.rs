// TODO(ran) FIXME: replace with envlogger crate

trait Logger {
    fn log_err(msg: &str);
}

struct StdErrLogger {}

impl Logger for StdErrLogger {
    fn log_err(msg: &str) {
        eprintln!("{}", msg)
    }
}
