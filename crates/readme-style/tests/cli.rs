//! End-to-end tests of the binary, run against temporary projects.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

const README: &str = "# demo\n\n> **Note:** hello\n";

fn project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    fs::create_dir(dir.path().join(".github")).expect("repo root marker");
    for (name, content) in files {
        let path = dir.path().join(name);
        fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        fs::write(path, content).expect("write");
    }
    dir
}

fn cli() -> Command {
    let mut command = Command::cargo_bin("readme-style").expect("binary");
    command.arg("--color").arg("never");
    command
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("readable")
}

#[test]
fn check_reports_and_fails_without_writing() {
    let dir = project(&[("README.md", README)]);
    cli().arg("--check").arg(dir.path()).assert().code(1).stdout(
        predicate::str::contains("+> [!NOTE]").and(predicate::str::contains("would be restyled (minimal)")),
    );
    assert_eq!(read(&dir.path().join("README.md")), README);
}

#[test]
fn write_then_check_passes() {
    let dir = project(&[("README.md", README)]);
    cli()
        .arg("--write")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("restyled (minimal)"));
    assert!(read(&dir.path().join("README.md")).contains("> [!NOTE]"));
    cli()
        .arg("--check")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("already styled"));
}

#[test]
fn the_default_never_writes_without_a_terminal() {
    let dir = project(&[("README.md", README)]);
    cli()
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("run with --write to apply"));
    assert_eq!(read(&dir.path().join("README.md")), README);
}

#[test]
fn stdout_prints_the_result_only() {
    let dir = project(&[("README.md", README)]);
    cli()
        .arg("--stdout")
        .arg(dir.path())
        .assert()
        .success()
        .stdout("# demo\n\n> [!NOTE]\n> hello\n");
}

#[test]
fn explain_shows_detected_facts_and_badge_reasons() {
    let dir = project(&[
        ("README.md", README),
        (
            "LICENSE",
            "MIT License\n\nPermission is hereby granted, free of charge",
        ),
    ]);
    cli()
        .args(["--explain", "--stdout"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("license      MIT (LICENSE)")
                .and(predicate::str::contains("badge        license")),
        );
}

#[test]
fn theme_comes_from_config_then_markers() {
    let dir = project(&[
        ("README.md", "# demo\n\n## A\n\n## B\n"),
        (".readme-style.toml", "theme = \"startup\"\n"),
    ]);
    cli()
        .arg("--write")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("(startup)"));
    fs::remove_file(dir.path().join(".readme-style.toml")).expect("remove config");
    cli()
        .arg("--check")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("already styled (startup)"));
}

#[test]
fn json_output_is_machine_readable() {
    let dir = project(&[("README.md", README)]);
    let output = cli()
        .args(["--format", "json", "--check"])
        .arg(dir.path())
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let reports: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
    assert_eq!(reports[0]["status"], "changed");
    assert_eq!(reports[0]["theme"], "minimal");
    assert_eq!(reports[0]["changed_by"][0], "callouts");
}

#[test]
fn errors_exit_with_two() {
    let dir = project(&[
        ("README.md", README),
        (".readme-style.toml", "theme = \"neon\"\n"),
    ]);
    cli()
        .arg("--check")
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown theme 'neon'"));
    cli()
        .arg("--check")
        .arg(dir.path().join("missing"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no README found"));
    cli()
        .args(["--theme", "neon", "--check"])
        .arg(dir.path())
        .assert()
        .code(2);
}

#[test]
fn several_files_report_the_worst_status() {
    let styled = project(&[("README.md", "# fine\n")]);
    let unstyled = project(&[("README.md", README)]);
    cli()
        .arg("--check")
        .arg(styled.path())
        .arg(unstyled.path())
        .assert()
        .code(1);
}

#[test]
fn crlf_files_stay_crlf() {
    let dir = project(&[("README.md", "# demo\r\n\r\n> **Note:** hello\r\n")]);
    cli().arg("--write").arg(dir.path()).assert().success();
    assert_eq!(
        read(&dir.path().join("README.md")),
        "# demo\r\n\r\n> [!NOTE]\r\n> hello\r\n"
    );
}

#[test]
fn list_themes() {
    cli().arg("--list-themes").assert().success().stdout(
        predicate::str::contains("minimal")
            .and(predicate::str::contains("ml-research"))
            .and(predicate::str::contains("docs")),
    );
}
