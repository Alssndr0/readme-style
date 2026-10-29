#!/usr/bin/env node
// Runs the readme-style binary from the platform package npm installed alongside this one.
"use strict";

const { spawnSync } = require("node:child_process");

const PACKAGES = {
  "linux-x64": "@readme-style/linux-x64",
  "linux-arm64": "@readme-style/linux-arm64",
  "darwin-x64": "@readme-style/darwin-x64",
  "darwin-arm64": "@readme-style/darwin-arm64",
  "win32-x64": "@readme-style/win32-x64",
  "win32-arm64": "@readme-style/win32-arm64",
};

const platform = `${process.platform}-${process.arch}`;
const name = PACKAGES[platform];
if (!name) {
  console.error(`readme-style: no prebuilt binary for ${platform}; install it with cargo or pip instead.`);
  process.exit(2);
}

let binary;
try {
  binary = require.resolve(`${name}/bin/readme-style${process.platform === "win32" ? ".exe" : ""}`);
} catch {
  console.error(`readme-style: the package ${name} is missing; reinstall without --no-optional.`);
  process.exit(2);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`readme-style: ${result.error.message}`);
  process.exit(2);
}
if (result.signal) {
  process.kill(process.pid, result.signal);
}
process.exit(result.status ?? 2);
