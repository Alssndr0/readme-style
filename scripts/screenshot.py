"""Render a README through GitHub's Markdown API and screenshot it with headless Chrome.

    python scripts/screenshot.py examples/python-lib/README.startup.md out.png

Needs the gh CLI (logged in) and Google Chrome or Chromium. The HTML is written next to
the README so relative image paths resolve. It uses the API's "gfm" mode, the only one that
renders alerts, which also turns single newlines inside paragraphs into line breaks.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

CSS = "https://cdn.jsdelivr.net/npm/github-markdown-css@5/github-markdown-light.css"
PAGE = """<!doctype html><html><head><meta charset="utf-8">
<link rel="stylesheet" href="{css}">
<style>body{{margin:0}} .markdown-body{{box-sizing:border-box;max-width:980px;margin:0 auto;padding:45px}}</style>
</head><body><article class="markdown-body">{body}</article></body></html>"""


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("readme", type=Path)
    parser.add_argument("png", type=Path)
    parser.add_argument("--height", type=int, default=1400)
    args = parser.parse_args()

    payload = json.dumps({"text": args.readme.read_text(encoding="utf-8"), "mode": "gfm"})
    body = subprocess.run(
        ["gh", "api", "markdown", "--input", "-"], input=payload, capture_output=True, text=True, check=True
    ).stdout
    html = args.readme.with_name(f".{args.readme.stem}.preview.html")
    html.write_text(PAGE.format(css=CSS, body=body), "utf-8")
    chrome = shutil.which("google-chrome") or shutil.which("chromium") or shutil.which("chromium-browser")
    if not chrome:
        sys.exit("Chrome or Chromium is required")
    subprocess.run(
        [chrome, "--headless", "--no-sandbox", "--hide-scrollbars", f"--screenshot={args.png.resolve()}",
         f"--window-size=1000,{args.height}", "--virtual-time-budget=8000", html.resolve().as_uri()],
        check=True, capture_output=True,
    )  # fmt: skip
    html.unlink()
    print(args.png)
    return 0


if __name__ == "__main__":
    sys.exit(main())
