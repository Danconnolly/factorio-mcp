use std::{env, path::PathBuf, process::Command};

fn fixture_path(variable: &str) -> PathBuf {
    let value = env::var_os(variable).unwrap_or_else(|| {
        panic!("{variable} is required: point it at the disposable walk/stop fixture runner")
    });
    let path = PathBuf::from(value);
    assert!(
        path.is_file(),
        "{variable} must name an existing file: {}",
        path.display()
    );
    path
}

/// Runs only against an operator-provided disposable server and copied save.
/// The runner must prove headless tick-driven walking and durable stop receipts;
/// it is not a real-player equivalence result.
#[test]
#[ignore = "requires a licensed Factorio headless walk/stop fixture and disposable save"]
fn walk_stop_end_to_end() {
    let runner = fixture_path("FACTORIO_WALK_FIXTURE");
    let save = fixture_path("FACTORIO_DISPOSABLE_SAVE");
    let status = Command::new(runner)
        .arg("--save")
        .arg(save)
        .arg("--assert-walk")
        .status()
        .expect("walk/stop fixture runner must start");
    assert!(
        status.success(),
        "walk/stop fixture runner reported failure: {status}"
    );
}
