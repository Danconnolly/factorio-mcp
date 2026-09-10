use std::{env, path::PathBuf, process::Command};

fn fixture_path(variable: &str) -> PathBuf {
    let value = env::var_os(variable).unwrap_or_else(|| {
        panic!("{variable} is required: point it at the disposable mining fixture runner")
    });
    let path = PathBuf::from(value);
    assert!(
        path.is_file(),
        "{variable} must name an existing file: {}",
        path.display()
    );
    path
}

/// Runs only against an operator-provided copied save and disposable server.
/// The runner uses fixed targets from the seed fixture and proves tick-driven
/// mining, reach, resource decrement, conservation, retry safety, and stop.
#[test]
#[ignore = "requires a licensed Factorio headless mining fixture and copied save"]
fn mine_end_to_end() {
    let runner = fixture_path("FACTORIO_MINE_FIXTURE");
    let save = fixture_path("FACTORIO_DISPOSABLE_SAVE");
    let status = Command::new(runner)
        .arg("--save")
        .arg(save)
        .arg("--assert-mine")
        .status()
        .expect("mining fixture runner must start");
    assert!(
        status.success(),
        "mining fixture runner reported failure: {status}"
    );
}
