use std::{env, path::PathBuf, process::Command};

fn fixture_path(variable: &str) -> PathBuf {
    let value = env::var_os(variable).unwrap_or_else(|| {
        panic!("{variable} is required: point it at the disposable lifecycle fixture runner")
    });
    let path = PathBuf::from(value);
    assert!(
        path.is_file(),
        "{variable} must name an existing file: {}",
        path.display()
    );
    path
}

/// Runs only against an operator-provided disposable Factorio fixture.
///
/// The fixture runner owns launching a licensed headless server and must verify
/// initialization, zero-client status, save/reload, restart, and human isolation.
/// It receives the disposable save and checked-in startup-setting fixture below.
#[test]
#[ignore = "requires a licensed Factorio headless lifecycle fixture and disposable save"]
fn actor_lifecycle_end_to_end() {
    let runner = fixture_path("FACTORIO_LIFECYCLE_FIXTURE");
    let save = fixture_path("FACTORIO_DISPOSABLE_SAVE");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let settings = root.join("tests/mod-settings/mod-settings.cfg");
    assert!(
        settings.is_file(),
        "documented mod-settings fixture is missing"
    );

    let status = Command::new(runner)
        .arg("--save")
        .arg(save)
        .arg("--mod-settings")
        .arg(settings)
        .status()
        .expect("lifecycle fixture runner must start");
    assert!(
        status.success(),
        "lifecycle fixture runner reported failure: {status}"
    );
}
