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
        .stdout(predicate::str::contains("restyled (minimal)").and(predicate::str::contains("+++").not()));
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

#[cfg(unix)]
#[test]
fn a_symlinked_readme_stays_a_symlink() {
    let dir = project(&[("docs/README.md", README)]);
    std::os::unix::fs::symlink("docs/README.md", dir.path().join("README.md")).expect("symlink");
    cli()
        .arg("--write")
        .arg(dir.path().join("README.md"))
        .assert()
        .success();
    assert!(
        fs::symlink_metadata(dir.path().join("README.md"))
            .expect("meta")
            .file_type()
            .is_symlink()
    );
    assert!(read(&dir.path().join("docs/README.md")).contains("> [!NOTE]"));
}

#[cfg(unix)]
#[test]
fn a_symlink_out_of_the_repository_is_not_written_through() {
    let outside = TempDir::new().expect("temp dir");
    let victim = outside.path().join("bashrc");
    fs::write(&victim, README).expect("victim");
    let dir = project(&[]);
    std::os::unix::fs::symlink(&victim, dir.path().join("README.md")).expect("symlink");
    cli()
        .arg("--write")
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("outside"));
    assert_eq!(read(&victim), README);
}

#[cfg(unix)]
#[test]
fn links_never_lead_out_of_the_repository() {
    let outside = TempDir::new().expect("temp dir");
    fs::write(outside.path().join("README.md"), README).expect("outside readme");
    fs::write(outside.path().join("secret.env"), "API_TOKEN=do-not-print\n").expect("secret");
    // A folder link above the README.
    let dir = project(&[]);
    std::os::unix::fs::symlink(outside.path(), dir.path().join("docs")).expect("folder link");
    cli()
        .arg("--write")
        .arg(dir.path().join("docs"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("outside the repository"));
    assert_eq!(read(&outside.path().join("README.md")), README);
    // Reading for a diff, too.
    let dir = project(&[]);
    std::os::unix::fs::symlink(outside.path().join("secret.env"), dir.path().join("README.md"))
        .expect("link");
    cli()
        .arg("--check")
        .arg(dir.path())
        .assert()
        .code(2)
        .stdout(predicate::str::contains("do-not-print").not())
        .stderr(predicate::str::contains("will not read"));
    // Even when the folder linked to is a repository of its own.
    fs::create_dir(outside.path().join(".git")).expect("outside repository");
    let dir = project(&[]);
    std::os::unix::fs::symlink(outside.path(), dir.path().join("docs")).expect("folder link");
    cli().arg("--check").arg(dir.path().join("docs")).assert().code(2);
    // Nor into any .git, nested ones included.
    let dir = project(&[("vendor/lib/.git/description", "nested\n")]);
    std::os::unix::fs::symlink("vendor/lib/.git/description", dir.path().join("README.md")).expect("link");
    cli().arg("--check").arg(dir.path()).assert().code(2);
    // And into .git.
    let dir = project(&[(".git/hooks/pre-commit", "#!/bin/sh\n")]);
    std::os::unix::fs::symlink(".git/hooks/pre-commit", dir.path().join("README.md")).expect("link");
    cli().arg("--write").arg(dir.path()).assert().code(2);
    assert_eq!(read(&dir.path().join(".git/hooks/pre-commit")), "#!/bin/sh\n");
}

#[cfg(unix)]
#[test]
fn outside_any_repository_the_readme_stays_in_its_folder() {
    let root = TempDir::new().expect("temp dir");
    fs::create_dir_all(root.path().join("home/.ssh")).expect("folders");
    fs::create_dir_all(root.path().join("download")).expect("folders");
    fs::write(root.path().join("home/.ssh/id_ed25519"), "SECRET KEY MATERIAL\n").expect("key");
    let readme = root.path().join("download/README.md");
    std::os::unix::fs::symlink("../home/.ssh/id_ed25519", &readme).expect("link");
    cli()
        .arg("--check")
        .arg(root.path().join("download"))
        .assert()
        .code(2)
        .stdout(predicate::str::contains("SECRET").not());
    // A folder link of the user's own, outside any repository, is followed.
    fs::create_dir_all(root.path().join("data/project")).expect("folders");
    fs::write(root.path().join("data/project/README.md"), README).expect("readme");
    std::os::unix::fs::symlink(root.path().join("data"), root.path().join("home/projects")).expect("link");
    cli()
        .arg("--check")
        .arg(root.path().join("home/projects/project"))
        .assert()
        .code(1);
}

#[test]
fn a_parent_folder_outside_the_current_repository_is_fine() {
    let parent = TempDir::new().expect("temp dir");
    fs::write(parent.path().join("README.md"), README).expect("readme");
    fs::create_dir_all(parent.path().join("repo/.git")).expect("inner repository");
    cli()
        .current_dir(parent.path().join("repo"))
        .args(["--check", ".."])
        .assert()
        .code(1);
}

#[cfg(unix)]
#[test]
fn a_read_only_readme_is_not_replaced() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = project(&[("README.md", README)]);
    let readme = dir.path().join("README.md");
    fs::set_permissions(&readme, fs::Permissions::from_mode(0o444)).expect("read-only");
    cli().arg("--write").arg(dir.path()).assert().code(2);
    assert_eq!(read(&readme), README);
}

#[cfg(unix)]
#[test]
fn a_planted_temporary_file_is_never_followed() {
    let outside = TempDir::new().expect("temp dir");
    let victim = outside.path().join("precious.txt");
    fs::write(&victim, "precious\n").expect("victim");
    let dir = project(&[("README.md", README)]);
    std::os::unix::fs::symlink(&victim, dir.path().join(".README.md.readme-style.tmp")).expect("symlink");
    cli().arg("--write").arg(dir.path()).assert().success();
    assert_eq!(read(&victim), "precious\n");
    assert!(read(&dir.path().join("README.md")).contains("> [!NOTE]"));
    assert!(
        !fs::symlink_metadata(dir.path().join("README.md"))
            .expect("meta")
            .file_type()
            .is_symlink()
    );
}

#[cfg(unix)]
#[test]
fn hard_links_stay_linked() {
    let dir = project(&[("README.md", README)]);
    fs::hard_link(dir.path().join("README.md"), dir.path().join("index.md")).expect("hard link");
    cli().arg("--write").arg(dir.path()).assert().success();
    let styled = read(&dir.path().join("README.md"));
    assert!(styled.contains("> [!NOTE]"));
    assert_eq!(read(&dir.path().join("index.md")), styled);
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .expect("list")
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn stdout_keeps_escape_bytes() {
    let source = "# demo\n\n```\n\u{1b}[32mgreen\u{1b}[0m\n```\n";
    let dir = project(&[("README.md", source)]);
    let output = cli()
        .arg("--stdout")
        .arg(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(output).expect("utf-8"), source);
}
