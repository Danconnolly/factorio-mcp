use std::{env, path::PathBuf, process::Command};

fn fixture_path(variable: &str) -> PathBuf {
    let value = env::var_os(variable).unwrap_or_else(|| {
        panic!("{variable} is required: point it at a disposable fixture runner")
    });
    let path = PathBuf::from(value);
    assert!(
        path.is_file(),
        "{variable} must name an existing file: {}",
        path.display()
    );
    path
}

/// Requires a licensed Factorio headless server, the production bridge mod, and
/// a copied save. The runner must prove local bounds, chart-denial before lookup,
/// deterministic sorting/truncation, and bridge-issued entity-ID access.
#[test]
#[ignore = "requires a licensed Factorio observation fixture and disposable save"]
fn observation_policy_end_to_end() {
    let runner = fixture_path("FACTORIO_OBSERVATION_FIXTURE");
    let save = fixture_path("FACTORIO_DISPOSABLE_SAVE");
    let status = Command::new(runner)
        .arg("--save")
        .arg(save)
        .arg("--assert-local-radius")
        .arg("32")
        .arg("--assert-max-results")
        .arg("128")
        .arg("--assert-max-payload-bytes")
        .arg((3 * 1024).to_string())
        .status()
        .expect("observation fixture runner must start");
    assert!(
        status.success(),
        "observation fixture runner reported failure: {status}"
    );
}
