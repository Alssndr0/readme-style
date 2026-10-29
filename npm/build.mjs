// Builds the npm packages for a release from the archives on the GitHub release.
//
//   node npm/build.mjs <version> <archives-dir> <out-dir>
//
// Writes <out-dir>/platforms/<name>/ (one package per platform, holding the binary) and
// <out-dir>/readme-style/ (the wrapper, depending on all of them as optional dependencies).

import { execFileSync } from "node:child_process";
import { chmodSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const TARGETS = [
  { target: "x86_64-unknown-linux-musl", name: "linux-x64", os: "linux", cpu: "x64" },
  { target: "aarch64-unknown-linux-musl", name: "linux-arm64", os: "linux", cpu: "arm64" },
  { target: "x86_64-apple-darwin", name: "darwin-x64", os: "darwin", cpu: "x64" },
  { target: "aarch64-apple-darwin", name: "darwin-arm64", os: "darwin", cpu: "arm64" },
  { target: "x86_64-pc-windows-msvc", name: "win32-x64", os: "win32", cpu: "x64", exe: true },
  { target: "aarch64-pc-windows-msvc", name: "win32-arm64", os: "win32", cpu: "arm64", exe: true },
];

const [version, archives, out] = process.argv.slice(2);
if (!version || !archives || !out) {
  console.error("usage: node npm/build.mjs <version> <archives-dir> <out-dir>");
  process.exit(2);
}

const here = dirname(fileURLToPath(import.meta.url));
const wrapper = JSON.parse(readFileSync(join(here, "readme-style", "package.json"), "utf8"));
rmSync(out, { recursive: true, force: true });

for (const { target, name, os, cpu, exe } of TARGETS) {
  const dir = join(out, "platforms", name);
  const bin = join(dir, "bin");
  mkdirSync(bin, { recursive: true });
  const archive = join(archives, `readme-style-${target}.${exe ? "zip" : "tar.gz"}`);
  if (!existsSync(archive)) {
    throw new Error(`missing ${archive}`);
  }
  if (exe) {
    execFileSync("unzip", ["-o", "-j", archive, "readme-style.exe", "-d", bin]);
  } else {
    execFileSync("tar", ["-xzf", archive, "-C", bin, "readme-style"]);
    chmodSync(join(bin, "readme-style"), 0o755);
  }
  const manifest = {
    name: `@readme-style/${name}`,
    version,
    description: `The readme-style binary for ${os} ${cpu}.`,
    license: wrapper.license,
    repository: wrapper.repository,
    os: [os],
    cpu: [cpu],
    files: ["bin"],
  };
  writeFileSync(join(dir, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  wrapper.optionalDependencies[manifest.name] = version;
}

const dir = join(out, "readme-style");
cpSync(join(here, "readme-style"), dir, { recursive: true });
cpSync(join(here, "..", "README.md"), join(dir, "README.md"));
cpSync(join(here, "..", "LICENSE"), join(dir, "LICENSE"));
wrapper.version = version;
writeFileSync(join(dir, "package.json"), `${JSON.stringify(wrapper, null, 2)}\n`);
console.log(`npm packages for ${version} in ${out}`);
