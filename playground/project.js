// A project folder chosen in the browser. Its files are read in this page and never uploaded.

const README_NAMES = ["README.md", "readme.md", "Readme.md", "README.markdown", "readme.markdown"];
/** Folders that never hold anything readme-style looks at, and can hold a great many files. */
const SKIP = new Set([
  ".git", ".hg", ".svn", "node_modules", "target", "dist", "build", ".venv", "venv", "__pycache__",
  ".tox", ".nox", ".mypy_cache", ".pytest_cache", ".ruff_cache", ".next", ".cache", "coverage",
  "vendor",
]);
/** Files at the top of the project whose content readme-style reads. */
const READ_AT_ROOT = new Set([
  "pyproject.toml", "package.json", "Cargo.toml", "go.mod", ".readme-style.toml",
  ".pre-commit-config.yaml", "ruff.toml", ".ruff.toml", "codecov.yml", ".codecov.yml",
  ".readthedocs.yaml", ".readthedocs.yml", "readthedocs.yml",
]);
/** Whether readme-style reads a file; any other file only needs to be known to exist. */
function isRead(path) {
  if (path === ".git/config" || /^\.github\/workflows\/[^/]+\.ya?ml$/i.test(path)) return true;
  return !path.includes("/") && (READ_AT_ROOT.has(path) || /^(licen[cs]e|copying|unlicense)/i.test(path));
}
const IMAGE = /\.(svg|png|jpe?g|gif|webp|avif)$/i;
const TEXT_LIMIT = 512 * 1024;
const TOTAL_TEXT_LIMIT = 8 * 1024 * 1024;
const IMAGE_LIMIT = 4 * 1024 * 1024;
const MAX_FILES = 20000;

/**
 * Read a folder from an `<input type="file" webkitdirectory>`: its README, the files
 * readme-style reads (by path inside the folder), and its images, for the preview.
 */
export async function readFolder(fileList) {
  const files = {};
  const images = new Map();
  let readme = null;
  let seen = 0;
  let read = 0;
  for (const file of fileList) {
    // The first part is the chosen folder's own name.
    const parts = file.webkitRelativePath.split("/").slice(1);
    const path = parts.join("/");
    if (!path) continue;
    if (path !== ".git/config" && parts.slice(0, -1).some((part) => SKIP.has(part))) continue;
    if (++seen > MAX_FILES) break;
    const rank = parts.length === 1 ? README_NAMES.indexOf(path) : -1;
    if (rank >= 0 && (readme === null || rank < readme.rank)) {
      readme = { path, rank, text: await file.text() };
      continue;
    }
    const wanted = isRead(path) && file.size <= TEXT_LIMIT && read + file.size <= TOTAL_TEXT_LIMIT;
    files[path] = wanted ? await file.text() : "";
    if (wanted) read += file.size;
    if (IMAGE.test(path) && file.size <= IMAGE_LIMIT) images.set(path, file);
  }
  return { kind: "folder", readme, files, images, truncated: seen > MAX_FILES };
}

/** An example from examples.json, in the same shape. Its images are SVG text. */
export function exampleProject(example) {
  const images = new Map();
  for (const [path, text] of Object.entries(example.files)) {
    if (/\.svg$/i.test(path)) images.set(path, new Blob([text], { type: "image/svg+xml" }));
  }
  return {
    kind: "example",
    readme: { path: "README.md", text: example.readme },
    files: example.files,
    images,
    truncated: false,
  };
}
