use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(name)
}

#[test]
fn info_emits_stable_json_contract() {
    let output = cargo_bin_cmd!("butterpaper-cli")
        .arg("info")
        .arg(fixture("small.pdf"))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let mut value: Value =
        serde_json::from_slice(&output).expect("stdout should contain valid json");
    value["path"] = Value::String("<FIXTURE>".to_owned());

    insta::assert_json_snapshot!("cli_info_small_pdf", value);
}

#[test]
fn open_supports_dry_run_for_tests() {
    cargo_bin_cmd!("butterpaper-cli")
        .arg("open")
        .arg(fixture("small.pdf"))
        .env("BUTTERPAPER_TEST_NO_SPAWN", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("open:"));
}

#[test]
fn render_thumb_writes_png_file() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let output_path = temp.path().join("thumb.png");

    cargo_bin_cmd!("butterpaper-cli")
        .arg("render-thumb")
        .arg(fixture("medium.pdf"))
        .arg("--page")
        .arg("2")
        .arg("--width")
        .arg("120")
        .arg("--height")
        .arg("120")
        .arg("--output")
        .arg(&output_path)
        .assert()
        .success();

    assert!(output_path.exists(), "thumbnail output file should exist");

    let image = image::open(&output_path).expect("thumbnail should be readable image");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
}

#[test]
fn info_fails_for_missing_file() {
    cargo_bin_cmd!("butterpaper-cli")
        .arg("info")
        .arg(fixture("missing.pdf"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("file does not exist"));
}

#[test]
fn info_fails_for_invalid_pdf() {
    cargo_bin_cmd!("butterpaper-cli")
        .arg("info")
        .arg(fixture("invalid.pdf"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to open PDF"));
}

#[test]
fn info_fails_for_encrypted_marker_pdf() {
    cargo_bin_cmd!("butterpaper-cli")
        .arg("info")
        .arg(fixture("encrypted-marker.pdf"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("encrypted PDFs are not supported"));
}
