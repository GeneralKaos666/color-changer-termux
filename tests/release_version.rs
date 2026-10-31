//! The release tag gate must accept one tag and one tag only: `v` followed by
//! the exact `[package] version` of `Cargo.toml`.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// Run `scripts/check-release-version.sh <tag>` against a temporary crate
/// manifest.
fn run_gate(tag: &str, manifest: &str) -> Output {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("Cargo.toml"), manifest).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/check-release-version.sh");
    Command::new("sh")
        .arg(script)
        .arg(tag)
        .current_dir(dir.path())
        .output()
        .unwrap()
}

#[test]
fn exact_version_matches() {
    let output = run_gate("v1.1.1", "[package]\nname = \"x\"\nversion = \"1.1.1\"\n");
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn prefix_tag_fails() {
    let output = run_gate("v1.1.1", "[package]\nname = \"x\"\nversion = \"1.1.10\"\n");
    assert!(!output.status.success(), "a prefix must not match");
}

#[test]
fn different_version_fails() {
    let output = run_gate("v2.0.0", "[package]\nname = \"x\"\nversion = \"1.1.1\"\n");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not match"), "{stderr}");
}

#[test]
fn dependency_version_is_ignored() {
    let manifest =
        "[package]\nname = \"x\"\nversion = \"1.2.3\"\n\n[dependencies.foo]\nversion = \"9.9.9\"\n";
    let output = run_gate("v1.2.3", manifest);
    assert!(output.status.success(), "{output:?}");
}

/// Run `scripts/crate-version.sh` against a temporary crate manifest.
fn run_version(manifest: &str) -> Output {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("Cargo.toml"), manifest).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/crate-version.sh");
    Command::new("sh")
        .arg(script)
        .current_dir(dir.path())
        .output()
        .unwrap()
}

#[test]
fn version_script_prints_the_package_version() {
    let output = run_version("[package]\nname = \"x\"\nversion = \"1.2.3\"\n");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1.2.3");
}

#[test]
fn version_script_ignores_dependency_versions() {
    let manifest =
        "[package]\nname = \"x\"\nversion = \"1.2.3\"\n\n[dependencies.foo]\nversion = \"9.9.9\"\n";
    let output = run_version(manifest);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1.2.3");
}

#[test]
fn version_script_fails_without_a_package_version() {
    let output = run_version("[package]\nname = \"x\"\n");
    assert!(!output.status.success(), "an absent version must fail");
}
