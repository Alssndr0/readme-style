// The rendered README, shown in a sandboxed frame: the README's own HTML is displayed as GitHub
// would display it, and its scripts cannot run. Only the frame's own script, carrying a fresh
// nonce, may run; it scrolls to sections and hands other links to the page.

const FRAME_SCRIPT = `
const decode = (text) => { try { return decodeURIComponent(text); } catch { return text; } };
const scrollTarget = (id) =>
  document.getElementById(id) || document.getElementById("user-content-" + id);
document.addEventListener("click", (event) => {
  const link = event.target.closest && event.target.closest("a[href]");
  if (!link) return;
  event.preventDefault();
  const href = link.getAttribute("href");
  if (href.startsWith("#")) {
    const id = decode(href.slice(1));
    const target = id ? scrollTarget(id) : document.body;
    if (target) target.scrollIntoView({ behavior: "smooth" });
  } else {
    parent.postMessage({ readmeStyle: "open", href }, "*");
  }
});
addEventListener("scroll", () => parent.postMessage({ readmeStyle: "scroll", y: scrollY }, "*"), { passive: true });
addEventListener("DOMContentLoaded", () => scrollTo(0, INITIAL_SCROLL));
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

/** A path inside the project for a relative URL, as GitHub resolves it from the README. */
export function projectPath(url) {
  const parts = [];
  for (const part of url.trim().split(/[?#]/)[0].split("/")) {
    if (part === "" || part === ".") continue;
    if (part === "..") parts.pop();
    else parts.push(decode(part));
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
 * Point relative image URLs at something the frame can load: a file of the opened folder, or
 * the repository's raw file on GitHub. Parsing does not run scripts or load anything.
 */
async function resolveImages(html, { images, repo }) {
  const doc = new DOMParser().parseFromString(`<body>${html}</body>`, "text/html");
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
  return doc.body.innerHTML;
}

/** The frame's document: the rendered README, the Markdown styles and the frame script. */
export async function previewDocument(html, { css, images, repo, scroll }) {
  const n = nonce();
  const body = await resolveImages(html, { images, repo });
  const policy = [
    "default-src 'none'",
    "img-src https: http: data:",
    "media-src https: data:",
    "style-src 'unsafe-inline'",
    `script-src 'nonce-${n}'`,
  ].join("; ");
  const script = FRAME_SCRIPT.replace("INITIAL_SCROLL", String(Math.round(scroll) || 0));
  return `<!doctype html>
<html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="${policy}">
<style>${css}</style>
<script nonce="${n}">${script}</script>
</head><body><article class="markdown-body">${body}</article></body></html>`;
}

/** Where a link in the README leads, if anywhere outside the README itself. */
export function linkTarget(href, repo) {
  if (/^https?:\/\//i.test(href)) return href;
  if (isRelative(href) && repo) {
    return `https://github.com/${repo}/blob/HEAD/${projectPath(href)}`;
  }
  return null;
}
