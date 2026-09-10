use std::{env, path::PathBuf, process::Command};

fn fixture_path(variable: &str) -> PathBuf {
    let value = env::var_os(variable).unwrap_or_else(|| {
        panic!("{variable} is required: point it at the disposable crafting fixture runner")
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
/// The retained empty-inventory seed proves whether a zero-client virtual actor
/// exposes the native queue required before crafting can be promoted.
#[test]
#[ignore = "requires a licensed Factorio headless crafting fixture and copied save"]
fn craft_zero_start_end_to_end() {
    let runner = fixture_path("FACTORIO_CRAFT_FIXTURE");
    let save = fixture_path("FACTORIO_DISPOSABLE_SAVE");
    let status = Command::new(runner)
        .arg("--save")
        .arg(save)
        .arg("--assert-craft")
        .status()
        .expect("crafting fixture runner must start");
    assert!(
        status.success(),
        "crafting fixture runner reported failure: {status}"
    );
}
