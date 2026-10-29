"""Smoke test of the built playground in headless Chrome, with no dependencies.

    python3 -I playground/tests/smoke.py target/playground

Serves the folder, opens the page and checks that every example styles with every theme, that
typing restyles, and that a README's own scripts never run. Chrome is driven over
--remote-debugging-pipe; set CHROME to its path if it is not on PATH.
"""

import base64
import functools
import http.server
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time


class Chrome:
    """Just enough of the DevTools protocol: one page, commands, script evaluation."""

    def __init__(self, binary, profile):
        to_chrome, self._write = os.pipe()
        self._read, from_chrome = os.pipe()

        def pipes_on_fds_3_and_4():
            fds = [os.dup(to_chrome), os.dup(from_chrome)]
            os.dup2(fds[0], 3)
            os.dup2(fds[1], 4)
            os.set_inheritable(3, True)
            os.set_inheritable(4, True)

        self.process = subprocess.Popen(
            [binary, "--headless=new", "--remote-debugging-pipe", "--no-first-run",
             "--no-default-browser-check", "--disable-gpu", "--no-sandbox",
             f"--user-data-dir={profile}", "about:blank"],
            preexec_fn=pipes_on_fds_3_and_4, close_fds=False,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        os.close(to_chrome)
        os.close(from_chrome)
        self._next_id = 0
        self._buffer = b""
        self.events = []
        page = next(t for t in self.call("Target.getTargets")["targetInfos"] if t["type"] == "page")
        self.session = self.call("Target.attachToTarget", targetId=page["targetId"], flatten=True)["sessionId"]
        self.call("Runtime.enable", page=True)
        self.call("Emulation.setDeviceMetricsOverride", page=True, width=1400, height=900,
                  deviceScaleFactor=1, mobile=False)

    def _message(self):
        while b"\0" not in self._buffer:
            chunk = os.read(self._read, 1 << 20)
            if not chunk:
                raise RuntimeError("Chrome exited")
            self._buffer += chunk
        message, self._buffer = self._buffer.split(b"\0", 1)
        return json.loads(message)

    def call(self, method, page=False, **params):
        self._next_id += 1
        message = {"id": self._next_id, "method": method, "params": params}
        if page:
            message["sessionId"] = self.session
        os.write(self._write, json.dumps(message).encode() + b"\0")
        while True:
            reply = self._message()
            if reply.get("id") == self._next_id:
                if "error" in reply:
                    raise RuntimeError(f"{method}: {reply['error']}")
                return reply.get("result", {})
            self.events.append(reply)

    def eval(self, expression):
        result = self.call("Runtime.evaluate", page=True, expression=expression,
                           awaitPromise=True, returnByValue=True)
        if "exceptionDetails" in result:
            raise RuntimeError(json.dumps(result["exceptionDetails"])[:500])
        return result["result"].get("value")

    def wait(self, expression, timeout=20):
        deadline = time.time() + timeout
        while time.time() < deadline:
            if self.eval(expression):
                return
            time.sleep(0.05)
        raise TimeoutError(expression)

    def exceptions(self):
        return [e["params"]["exceptionDetails"] for e in self.events
                if e.get("method") == "Runtime.exceptionThrown"]

    def screenshot(self, path):
        data = self.call("Page.captureScreenshot", page=True, format="png")["data"]
        with open(path, "wb") as file:
            file.write(base64.b64decode(data))

    def close(self):
        self.process.kill()
        self.process.wait()


REQUESTED = []


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def send_head(self):
        REQUESTED.append(self.path)
        return super().send_head()


def serve(folder):
    handler = functools.partial(QuietHandler, directory=folder)
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, f"http://127.0.0.1:{server.server_address[1]}/"


SET = """((id, value, event) => {
  const status = document.getElementById("status");
  const updated = new Promise((resolve) => {
    new MutationObserver((_, observer) => {
      observer.disconnect();
      resolve();
    }).observe(status, { attributes: true, childList: true, characterData: true, subtree: true });
  });
  const element = document.getElementById(id);
  element.value = value;
  element.dispatchEvent(new Event(event));
  return updated.then(() => [status.className.replace("status ", ""), status.textContent]);
})"""
STATUS = "document.getElementById('status')"


def restyle(chrome, id, value, event):
    """Change a control, wait for the result to be shown, and return the status."""
    return chrome.eval(f"{SET}({json.dumps(id)}, {json.dumps(value)}, {json.dumps(event)})")


def main():
    folder = sys.argv[1] if len(sys.argv) > 1 else "target/playground"
    binary = os.environ.get("CHROME") or next(
        (b for b in ("google-chrome", "chromium", "chromium-browser") if shutil.which(b)), None)
    if not binary:
        sys.exit("Chrome or Chromium is required (or set CHROME)")
    failures = []
    server, url = serve(folder)
    with tempfile.TemporaryDirectory() as profile:
        chrome = Chrome(binary, profile)
        try:
            chrome.call("Page.navigate", page=True, url=url)
            chrome.wait(f"{STATUS} && /changed|unchanged/.test({STATUS}.className)")
            examples = chrome.eval("[...document.querySelectorAll('#example option')].map(o => o.value).filter(Boolean)")
            themes = chrome.eval("[...document.querySelectorAll('#theme option')].map(o => o.value).filter(Boolean)")
            if len(examples) < 4 or len(themes) < 5:
                failures.append(f"expected the examples and themes, got {examples} and {themes}")
            for example in examples:
                restyle(chrome, "example", example, "change")
                for theme in themes:
                    status, text = restyle(chrome, "theme", theme, "change")
                    if status not in ("changed", "unchanged") or theme not in text:
                        failures.append(f"example {example}, theme {theme}: {status}: {text}")
            restyle(chrome, "theme", "", "change")
            status, _ = restyle(chrome, "source", "# typed\n\n> **Note:** typed text\n", "input")
            output = chrome.eval("document.getElementById('output').textContent")
            if status != "changed" or "> [!NOTE]" not in output:
                failures.append(f"typing did not restyle: {status}: {output[:200]}")
            hostile = "\n\n".join([
                "# demo",
                '<img src=x onerror="parent.document.title=\'ran\'">',
                '<script>parent.document.title = "ran"</script>',
                '[x](javascript:parent.document.title="ran")',
                '<meta http-equiv="refresh" content="0;url=/evil-refresh">',
                '<base href="/evil-base/">',
                '<img src="relative.png">',
                '<svg><a xlink:href="/evil-svg"><text y="20">svg</text></a></svg>',
                '<div><template shadowrootmode="open"><a href="/evil-shadow">shadow</a></template></div>',
                '<map name="m"><area href="/evil-area" shape="rect" coords="0,0,9,9"></map>',
                '<a href="https://example.org/ok" style="color:red" onclick="x()">kept</a>',
            ]) + "\n"
            restyle(chrome, "source", hostile, "input")
            time.sleep(1.5)
            if chrome.eval("document.title") != "readme-style playground":
                failures.append("a script in the README ran")
            evil = [path for path in REQUESTED if "evil" in path]
            if evil:
                failures.append(f"the preview requested {evil}")
            srcdoc = chrome.eval("document.getElementById('preview').srcdoc")
            body = srcdoc.split("<body>", 1)[-1]
            for marker in ("<meta", "<base", "<svg", "<template", "<area", " onclick=", " onerror=", " style=", "javascript:"):
                if marker in body:
                    failures.append(f"the preview kept {marker}")
            if 'href="https://example.org/ok"' not in body:
                failures.append("a safe link was dropped")
            for exception in chrome.exceptions():
                failures.append(f"uncaught exception: {json.dumps(exception)[:300]}")
            if failures:
                chrome.screenshot("playground-failure.png")
        finally:
            chrome.close()
            server.shutdown()
    for failure in failures:
        print(f"FAIL {failure}")
    print(f"{len(failures)} failures" if failures else "playground smoke test passed")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
