// The rendered README, shown in a sandboxed frame as GitHub would show it. The HTML is first
// reduced to what GitHub keeps (sanitize.js); the frame then has an opaque origin and a content
// security policy under which only its own script, carrying a fresh nonce, may run. That script
// scrolls to sections and hands other links to the page, tagged with a token only this document
// knows, so nothing else in the frame can speak for it.

import { sanitize } from "./sanitize.js";

const FRAME_SCRIPT = `
const TOKEN = FRAME_TOKEN;
const tell = (message) => parent.postMessage({ ...message, token: TOKEN }, "*");
const decode = (text) => { try { return decodeURIComponent(text); } catch { return text; } };
// Taken before the README's own elements exist, which could otherwise shadow them by name.
const byId = Document.prototype.getElementById.bind(document);
const byName = Document.prototype.getElementsByName.bind(document);
const scrollTarget = (id) =>
  byId("user-content-" + id) || byId(id) || byName("user-content-" + id)[0] || null;
const follow = (event) => {
  // The main button, or the middle one (which would open a tab); never the right one.
  if (event.button !== 0 && event.button !== 1) return;
  const link = event.composedPath().find(
    (node) => node instanceof Element && (node.localName === "a" || node.localName === "area"));
  if (!link) return;
  event.preventDefault();
  const href = link.getAttribute("href") ?? "";
  if (href.startsWith("#")) {
    const id = decode(href.slice(1));
    const target = id ? scrollTarget(id) : null;
    if (target) target.scrollIntoView({ behavior: "smooth" });
    else if (!id || id.toLowerCase() === "top") scrollTo({ top: 0, behavior: "smooth" });
  } else if (href) {
    tell({ readmeStyle: "open", href });
  }
};
document.addEventListener("click", follow, true);
document.addEventListener("auxclick", follow, true);
addEventListener("scroll", () => tell({ readmeStyle: "scroll", y: scrollY }), { passive: true });
// Back to where the reader was. A new frame has no size yet when its document is parsed, and
// images make it taller as they load: keep trying until there, unless the reader scrolls first.
let target = INITIAL_SCROLL;
let settled = target === 0;
const restore = () => {
  if (settled) return;
  scrollTo(0, target);
  settled = Math.abs(scrollY - target) < 2;
};
for (const name of ["wheel", "keydown", "touchstart", "mousedown"]) {
  addEventListener(name, () => { settled = true; }, { passive: true, capture: true });
}
for (const name of ["resize", "load"]) addEventListener(name, restore);
addEventListener("message", (event) => {
  const data = event.data;
  if (event.source !== parent || !data || data.token !== TOKEN || data.readmeStyle !== "scroll") return;
  target = Math.max(0, Number(data.y) || 0);
  settled = false;
  restore();
});
addEventListener("DOMContentLoaded", () => {
  restore();
  tell({ readmeStyle: "ready" });
});
// Leaving this document (something navigated the frame) is worth knowing even when the next page
// never finishes loading.
addEventListener("pagehide", () => tell({ readmeStyle: "leaving" }));
`;

function nonce() {
  const bytes = crypto.getRandomValues(new Uint8Array(18));
  return btoa(String.fromCharCode(...bytes));
}

function decode(text) {
  try {
    return decodeURIComponent(text);
  } catch {
    return text;
  }
}

function isRelative(url) {
  return url && !/^([a-z][a-z0-9+.-]*:|\/\/|#)/i.test(url.trim());
}

/**
 * A path inside the project for a relative URL, as GitHub resolves it from the README: decoded,
 * to find a file, or still encoded, to build another URL (`a%23b.md` names `a#b.md`).
 */
export function projectPath(url, { decoded = true } = {}) {
  const parts = [];
  for (const part of url.trim().split(/[?#]/)[0].split("/")) {
    if (part === "" || part === ".") continue;
    if (part === "..") parts.pop();
    else parts.push(decoded ? decode(part) : part);
  }
  return parts.join("/");
}

const dataUrls = new WeakMap();

function dataUrl(file) {
  if (!dataUrls.has(file)) {
    dataUrls.set(
      file,
      new Promise((resolve) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result);
        reader.onerror = () => resolve(null);
        reader.readAsDataURL(file);
      }),
    );
  }
  return dataUrls.get(file);
}

/**
 * Point relative image URLs in `doc` at something the frame can load: a file of the opened
 * folder, or the repository's raw file on GitHub.
 */
async function resolveImages(doc, { images, repo }) {
  const resolve = async (url) => {
    if (!isRelative(url)) return url;
    const path = projectPath(url);
    const file = images.get(path);
    if (file) return (await dataUrl(file)) ?? url;
    return repo ? `https://raw.githubusercontent.com/${repo}/HEAD/${path}` : url;
  };
  for (const img of doc.querySelectorAll("img[src]")) {
    img.setAttribute("src", await resolve(img.getAttribute("src")));
  }
  for (const source of doc.querySelectorAll("source[srcset]")) {
    const candidates = await Promise.all(
      source
        .getAttribute("srcset")
        .split(",")
        .map(async (candidate) => {
          const [url, ...size] = candidate.trim().split(/\s+/);
          return [await resolve(url), ...size].join(" ");
        }),
    );
    source.setAttribute("srcset", candidates.join(", "));
  }
}

/**
 * The frame's document for rendered README HTML: sanitized, with images resolved, the Markdown
 * styles and the frame script. `token` identifies the frame's messages.
 */
export async function previewDocument(html, { css, images, repo, scroll }) {
  const n = nonce();
  const token = nonce();
  const doc = sanitize(html);
  await resolveImages(doc, { images, repo });
  // Links show where they lead, so opening one in a new tab goes to the same place.
  for (const link of doc.querySelectorAll("a[href]")) {
    const target = linkTarget(link.getAttribute("href"), repo);
    if (target) link.setAttribute("href", target);
  }
  const policy = [
    "default-src 'none'",
    "img-src https: http: data:",
    // Like GitHub, which plays only videos it hosts.
    "media-src https://github.com https://*.githubusercontent.com",
    "style-src 'unsafe-inline'",
    `script-src 'nonce-${n}'`,
    "base-uri 'none'",
    "form-action 'none'",
  ].join("; ");
  const script = FRAME_SCRIPT.replace("FRAME_TOKEN", JSON.stringify(token)).replace(
    "INITIAL_SCROLL",
    String(Math.round(scroll) || 0),
  );
  const page = `<!doctype html>
<html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="${policy}">
<style>${css}</style>
<script nonce="${n}">${script}</script>
</head><body><article class="markdown-body">${doc.body.innerHTML}</article></body></html>`;
  return { page, token };
}

/** Where a link in the README leads, if anywhere outside the README itself. */
export function linkTarget(href, repo) {
  if (/^https?:\/\//i.test(href)) return href;
  if (isRelative(href) && repo) {
    const suffix = href.match(/[?#].*$/)?.[0] ?? "";
    return `https://github.com/${repo}/blob/HEAD/${projectPath(href, { decoded: false })}${suffix}`;
  }
  return null;
}
