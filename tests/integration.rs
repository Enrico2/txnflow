use assert_cmd::Command;
use csv::Trim;
use predicates::prelude::*;

use txnflow::OutputAccount;

#[test]
fn integration_test() {
    let mut cmd = Command::cargo_bin("txnflow").unwrap();
    let result = cmd.arg("tests/test_data.csv").assert();

    result.stdout(predicate::function(|stdout_slice: &[u8]| {
        // let stdout_slice = result.stdout.as_slice();

        let mut actual: Vec<OutputAccount> = vec![];
        let mut reader = csv::ReaderBuilder::new()
            .trim(Trim::All)
            .from_reader(stdout_slice);
        for r in reader.deserialize() {
            actual.push(r.unwrap());
        }
        let expected_csv = include_str!("expected_result.csv");
        let mut expected: Vec<OutputAccount> = vec![];
        let mut reader = csv::ReaderBuilder::new()
            .trim(Trim::All)
            .from_reader(expected_csv.as_bytes());
        for r in reader.deserialize() {
            let r: OutputAccount = r.unwrap();
            expected.push(r);
        }
        actual.sort_by(|a, b| a.client.cmp(&b.client));
        expected.sort_by(|a, b| a.client.cmp(&b.client));
        actual == expected
    }));
}
