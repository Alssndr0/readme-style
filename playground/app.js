// The playground: a README on the left, the engine's result on the right.

import { Engine } from "./engine.js";
import { linkTarget, previewDocument } from "./preview.js";
import { exampleProject, readFolder } from "./project.js";

const $ = (id) => document.getElementById(id);
const ui = {
  source: $("source"),
  example: $("example"),
  folder: $("folder"),
  theme: $("theme"),
  repo: $("repo"),
  scheme: $("scheme"),
  path: $("readme-path"),
  project: $("project"),
  status: $("status"),
  frame: $("preview"),
  diff: $("diff"),
  output: $("output"),
  details: $("details"),
  copy: $("copy"),
  download: $("download"),
  tabs: [...document.querySelectorAll('[role="tab"]')],
  views: [...document.querySelectorAll("[data-view]")],
};

const state = {
  engine: null,
  css: "",
  project: null,
  result: null,
  tab: "styled",
  scroll: { styled: 0, original: 0 },
  shown: null,
  /** The token of the document in the preview frame; its messages carry it. */
  frameToken: null,
  /** Whether the frame's next load is one the page started. */
  expectLoad: false,
};

const STOPPED = `<!doctype html><meta http-equiv="Content-Security-Policy" content="default-src 'none'">
<p>The README tried to replace the preview with another page, so it was stopped.
GitHub would remove what did that.</p>`;

function element(tag, text, className) {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
}

function debounce(fn, ms) {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

// Styling

function restyle() {
  const project = state.project;
  const request = {
    readme: ui.source.value,
    path: project.readme.path,
    files: project.files,
    theme: ui.theme.value || null,
    repo: ui.repo.value.trim() || null,
  };
  try {
    state.result = state.engine.style(request);
  } catch (error) {
    state.result = { status: "error", message: error.message, changed_by: [], badges: [], diagnostics: [], warnings: [] };
  }
  showStatus();
  showDiff();
  showMarkdown();
  showDetails();
  showPreview();
}

function showStatus() {
  const { status, theme, theme_source: source } = state.result;
  const labels = {
    changed: `restyled with ${theme}`,
    unchanged: `already styled (${theme})`,
    refused: "refused: the content would change",
    error: "error",
  };
  ui.status.className = `status ${status}`;
  ui.status.textContent = labels[status];
  ui.status.title = state.result.message ?? (source ? `theme from ${source}` : "");
}

function showDiff() {
  const { diff, status, message } = state.result;
  ui.diff.replaceChildren();
  if (!diff) {
    ui.diff.textContent = status === "unchanged" ? "Nothing to change: the README is already styled." : message ?? "";
    return;
  }
  for (const line of diff.split("\n")) {
    const kind = line.startsWith("+++") || line.startsWith("---")
      ? "head"
      : line.startsWith("@@") ? "hunk" : line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : undefined;
    ui.diff.append(element("span", `${line}\n`, kind));
  }
}

function showMarkdown() {
  ui.output.textContent = state.result.output ?? state.result.message ?? "";
  ui.copy.disabled = ui.download.disabled = !state.result.output;
}

function section(title, ...content) {
  return [element("h2", title), ...content];
}

function list(items) {
  const ul = element("ul");
  for (const item of items) ul.append(element("li", item));
  return ul;
}

function showDetails() {
  const r = state.result;
  const parts = [];
  if (r.message) parts.push(...section(r.status === "refused" ? "Why nothing was changed" : "Error", element("p", r.message, "message")));
  if (r.theme) parts.push(...section("Theme", element("p", `${r.theme}, ${r.theme_source === "chosen" ? "chosen above" : `from ${r.theme_source}`}`)));
  if (r.changed_by.length) parts.push(...section("Steps that changed something", list(r.changed_by)));
  if (r.badges.length) {
    const table = element("table");
    const head = element("tr");
    head.append(element("th", "Badge"), element("th", "Why"));
    table.append(head);
    for (const badge of r.badges) {
      const row = element("tr");
      const name = element("td");
      name.append(element("code", badge.kind));
      row.append(name, element("td", badge.reason));
      table.append(row);
    }
    parts.push(...section("Badges", table));
  }
  if (r.diagnostics.length) parts.push(...section("Problems in the README", list(r.diagnostics.map((d) => `line ${d.line}: ${d.message}`))));
  if (r.warnings.length) parts.push(...section("Warnings", list(r.warnings)));
  const read = Object.entries(state.project.files).filter(([, content]) => content).map(([path]) => path);
  parts.push(...section("Project files read", read.length ? list(read) : element("p", "None: open a project folder to infer badges from its files.")));
  ui.details.replaceChildren(...parts);
}

// The preview

async function showPreview() {
  if (state.tab !== "styled" && state.tab !== "original") return;
  const markdown = state.tab === "styled" ? state.result.output ?? ui.source.value : ui.source.value;
  const key = `${state.tab}\u0000${ui.repo.value}\u0000${markdown}`;
  if (key === state.shown) return;
  state.shown = key;
  const html = state.engine.render(markdown);
  const { page, token } = await previewDocument(html, {
    css: state.css,
    images: state.project.images,
    repo: ui.repo.value.trim() || null,
    scroll: state.scroll[state.tab],
  });
  if (state.shown !== key) return;
  state.frameToken = token;
  state.expectLoad = true;
  ui.frame.srcdoc = page;
}

// A load the page did not start means something in the frame navigated it away.
ui.frame.addEventListener("load", () => {
  if (state.expectLoad || state.frameToken === null) {
    state.expectLoad = false;
    return;
  }
  state.frameToken = null;
  state.shown = null;
  state.expectLoad = true;
  ui.frame.srcdoc = STOPPED;
});

window.addEventListener("message", (event) => {
  if (event.source !== ui.frame.contentWindow || typeof event.data !== "object" || event.data === null) return;
  if (state.frameToken === null || event.data.token !== state.frameToken) return;
  if (event.data.readmeStyle === "scroll" && typeof event.data.y === "number") {
    if (state.tab in state.scroll) state.scroll[state.tab] = event.data.y;
  } else if (event.data.readmeStyle === "open" && typeof event.data.href === "string") {
    const url = linkTarget(event.data.href, ui.repo.value.trim() || null);
    if (url) window.open(url, "_blank", "noopener,noreferrer");
  }
});

function selectTab(tab) {
  state.tab = tab;
  for (const button of ui.tabs) button.setAttribute("aria-selected", String(button.dataset.tab === tab));
  for (const view of ui.views) view.hidden = !view.dataset.view.split(" ").includes(tab);
  showPreview();
}

// Projects

function load(project, label) {
  state.project = project;
  state.scroll = { styled: 0, original: 0 };
  ui.source.value = project.readme.text;
  ui.path.textContent = project.readme.path;
  const count = Object.keys(project.files).length;
  ui.project.textContent = `${label}${count ? `, ${count} project files` : ""}${project.truncated ? " (first 20,000)" : ""}`;
  restyle();
}

/** A README pasted over a whole example is the user's own: the example's files no longer apply. */
function detachFromExample() {
  if (state.project.kind !== "example") return;
  state.project = { kind: "pasted", readme: { path: "README.md" }, files: {}, images: new Map(), truncated: false };
  ui.example.value = "";
  ui.path.textContent = "README.md";
  ui.project.textContent = "your README, without project files: open its folder to infer badges";
  restyle();
}

async function openFolder(files) {
  if (!files.length) return;
  ui.status.className = "status";
  ui.status.textContent = "Reading the folder…";
  const project = await readFolder(files);
  if (!project.readme) {
    ui.status.className = "status error";
    ui.status.textContent = "no README.md at the top of that folder";
    return;
  }
  ui.example.value = "";
  load(project, files[0].webkitRelativePath.split("/")[0]);
}

// Start

async function main() {
  try {
    const [engine, examples, css] = await Promise.all([
      Engine.load("readme_style_wasm.wasm"),
      fetch("examples.json").then((r) => r.json()),
      fetch("markdown.css").then((r) => r.text()),
    ]);
    state.engine = engine;
    state.css = css;
    for (const theme of engine.themes()) {
      const option = element("option", theme.name);
      option.value = theme.name;
      option.title = theme.description;
      ui.theme.append(option);
    }
    ui.example.append(Object.assign(element("option", "Your own"), { value: "", hidden: true }));
    for (const [i, example] of examples.entries()) {
      const option = element("option", example.name);
      option.value = String(i);
      option.title = example.about;
      ui.example.append(option);
    }
    ui.example.addEventListener("change", () => {
      const example = examples[Number(ui.example.value)];
      if (example) load(exampleProject(example), `${example.name} example`);
    });
    ui.folder.addEventListener("change", () => openFolder([...ui.folder.files]));
    ui.source.addEventListener("input", debounce(restyle, 150));
    ui.source.addEventListener("paste", () => {
      // Pasting over everything (or into an empty editor) replaces the README.
      const { selectionStart, selectionEnd, value } = ui.source;
      if (selectionStart === 0 && selectionEnd === value.length) setTimeout(detachFromExample);
    });
    ui.repo.addEventListener("input", debounce(restyle, 300));
    ui.theme.addEventListener("change", restyle);
    ui.scheme.addEventListener("change", () => {
      ui.frame.style.colorScheme = ui.scheme.value;
    });
    ui.scheme.value = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    ui.frame.style.colorScheme = ui.scheme.value;
    for (const button of ui.tabs) button.addEventListener("click", () => selectTab(button.dataset.tab));
    ui.copy.addEventListener("click", async () => {
      await navigator.clipboard.writeText(state.result.output);
      ui.copy.textContent = "Copied";
      setTimeout(() => (ui.copy.textContent = "Copy"), 1500);
    });
    ui.download.addEventListener("click", () => {
      const link = element("a");
      link.href = URL.createObjectURL(new Blob([state.result.output], { type: "text/markdown" }));
      link.download = state.project.readme.path.split("/").pop();
      link.click();
      setTimeout(() => URL.revokeObjectURL(link.href), 1000);
    });
    ui.example.value = "0";
    load(exampleProject(examples[0]), `${examples[0].name} example`);
  } catch (error) {
    ui.status.className = "status error";
    ui.status.textContent = `cannot start: ${error.message}`;
  }
}

main();
