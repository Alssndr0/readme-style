from readme_style.cli import main

README = "# demo\n\n> **Note:** hello\n"


def test_check_reports_and_fails(tree, capsys):
    path = tree({"README.md": README})
    assert main([str(path), "--check", "--color", "never"]) == 1
    out = capsys.readouterr().out
    assert "+> [!NOTE]" in out and "would be restyled" in out
    assert path.read_text() == README


def test_write_then_check_passes(tree, capsys):
    path = tree({"README.md": README})
    assert main([str(path), "--write"]) == 0
    assert "> [!NOTE]" in path.read_text()
    assert main([str(path), "--check"]) == 0
    assert "already styled" in capsys.readouterr().out


def test_default_mode_never_writes_without_a_tty(tree, capsys):
    path = tree({"README.md": README})
    assert main([str(path.parent)]) == 0
    assert "--write" in capsys.readouterr().out
    assert path.read_text() == README


def test_stdout_and_explain(tree, capsys):
    path = tree({"README.md": README, "LICENSE": "MIT License\nPermission is hereby granted, free of charge"})
    assert main([str(path), "--stdout", "--explain"]) == 0
    out = capsys.readouterr().out
    assert "license      MIT (LICENSE)" in out
    assert "> [!NOTE]" in out


def test_errors(tree, capsys, tmp_path):
    assert main([str(tmp_path / "missing")]) == 2
    path = tree({"README.md": README, ".readme-style.toml": 'theme = "neon"\n'})
    assert main([str(path), "--check"]) == 2
    assert "theme must be one of" in capsys.readouterr().err


def test_list_themes(capsys):
    assert main(["--list-themes", "--color", "never"]) == 0
    out = capsys.readouterr().out
    assert "minimal" in out and "startup" in out and "ml-research" in out
