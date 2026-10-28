"""Run every theme over a set of real READMEs and report guard failures, crashes and idempotency.

    python scripts/smoke.py PATH [PATH ...]

Each PATH is a README file or a folder holding one. Nothing is written.
"""

from __future__ import annotations

import sys
import time
import traceback
from pathlib import Path

from readme_style import THEMES, GuardError, IdempotencyError, get_theme, style_text
from readme_style.cli import find_readme
from readme_style.engine import read_text
from readme_style.project import load_project


def main(paths: list[str]) -> int:
    failures = 0
    for raw in paths:
        path = Path(raw)
        readme = find_readme(path) if path.is_dir() else path
        if readme is None:
            continue
        text = read_text(readme)
        project = load_project(readme)
        row = []
        for name in THEMES:
            start = time.perf_counter()
            try:
                result = style_text(text, get_theme(name), project)
                row.append(f"{name}:{'+' if result.changed else '='}{len(result.badges)}b")
            except (GuardError, IdempotencyError) as exc:
                failures += 1
                row.append(f"{name}:FAIL")
                print(f"  {readme} [{name}] {type(exc).__name__}: {exc}")
            except Exception:
                failures += 1
                row.append(f"{name}:CRASH")
                print(f"  {readme} [{name}] crashed:\n{traceback.format_exc()}")
            row[-1] += f" {1000 * (time.perf_counter() - start):.0f}ms"
        print(f"{str(readme.parent.name):40} {'  '.join(row)}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
