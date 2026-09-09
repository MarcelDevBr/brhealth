// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

use std::process::Command;

#[test]
fn test_cli_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_brhealth"))
        .arg("version")
        .output()
        .expect("Falha ao invocar binário da CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("BRHealth v"));
    assert!(stdout.contains("Hexagonal Data-Oriented Design"));
}

#[test]
fn test_cli_sources() {
    let output = Command::new(env!("CARGO_BIN_EXE_brhealth"))
        .arg("sources")
        .output()
        .expect("Falha ao invocar binário da CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("datasus.sim"));
    assert!(stdout.contains("ibge.censo"));
    assert!(stdout.contains("global.copernicus_era5"));
}

#[test]
fn test_cli_dv() {
    let output = Command::new(env!("CARGO_BIN_EXE_brhealth"))
        .args(["dv", "355030"])
        .output()
        .expect("Falha ao invocar binário da CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dígito Verificador (DV): 8"));
    assert!(stdout.contains("3550308"));
}

#[test]
fn test_cli_csap() {
    let output = Command::new(env!("CARGO_BIN_EXE_brhealth"))
        .args(["csap", "J45"])
        .output()
        .expect("Falha ao invocar binário da CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Grupo 7: Asma"));
}

#[test]
fn test_cli_roi() {
    let output = Command::new(env!("CARGO_BIN_EXE_brhealth"))
        .args([
            "roi",
            "--avoidable-cost",
            "1000000",
            "--investment",
            "200000",
            "--attributable-fraction",
            "0.5",
        ])
        .output()
        .expect("Falha ao invocar binário da CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("150.00%"));
    assert!(stdout.contains("ECONÔMICAMENTE SUPERAVITÁRIO"));
}
