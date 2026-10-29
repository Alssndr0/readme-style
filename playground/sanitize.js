// What GitHub keeps of a README's HTML, applied before the preview shows it. Modelled on the
// allowlist GitHub uses (html-pipeline's sanitization filter): known elements and attributes stay,
// links keep only safe protocols, anything else is unwrapped (its text kept) or, for elements whose
// content is never text, removed. The preview frame's sandbox and content security policy remain
// in place behind this.

const ELEMENTS = new Set([
  "a", "abbr", "b", "bdo", "blockquote", "br", "caption", "cite", "code", "dd", "del", "details",
  "dfn", "div", "dl", "dt", "em", "figcaption", "figure", "h1", "h2", "h3", "h4", "h5", "h6", "hr",
  "i", "img", "input", "ins", "kbd", "li", "mark", "ol", "p", "picture", "pre", "q", "rp", "rt",
  "ruby", "s", "samp", "section", "small", "source", "span", "strike", "strong", "sub", "summary",
  "sup", "table", "tbody", "td", "tfoot", "th", "thead", "time", "tr", "tt", "ul", "var", "video",
  "wbr",
]);

/** Elements removed with everything inside them. */
const REMOVE = new Set([
  "script", "style", "template", "svg", "math", "iframe", "frame", "frameset", "object", "embed",
  "noscript", "noembed", "noframes", "textarea", "select", "option", "title", "xmp", "head", "meta",
  "base", "link", "map", "area", "form", "button", "dialog", "slot", "canvas", "audio",
]);

const ATTRIBUTES = new Set([
  "abbr", "align", "alt", "aria-describedby", "aria-hidden", "aria-label", "aria-labelledby",
  "border", "cellpadding", "cellspacing", "colspan", "datetime", "dir", "height", "hspace", "id",
  "lang", "name", "nowrap", "open", "role", "rowspan", "scope", "span", "start", "summary",
  "title", "valign", "value", "vspace", "width",
]);

const PER_ELEMENT = {
  a: ["href", "data-footnote-ref", "data-footnote-backref"],
  img: ["src"],
  source: ["srcset", "media", "type"],
  ol: ["type"],
  input: ["type", "checked", "disabled"],
  video: ["src", "poster", "controls", "muted", "loop", "playsinline"],
};

/** Classes that the Markdown renderer itself produces, which the preview's styles rely on. */
const CLASS = /^(markdown-alert[\w-]*|anchor|footnotes|footnote-ref|footnote-backref|task-list-item|contains-task-list|data-footnote-backref)$/;

const PREFIX = "user-content-";

const SAFE_LINK = /^(https?:|mailto:|#|[^:]*$|[^:]*[/?#])/i;
const SAFE_SOURCE = /^(https?:|[^:]*$|[^:]*[/?#])/i;

function safeUrl(url, pattern) {
  return pattern.test(url.trim().replace(/[\u0000- ]/g, ""));
}

function clean(element) {
  for (const attribute of [...element.attributes]) {
    const name = attribute.name.toLowerCase();
    const tag = element.localName;
    let keep = ATTRIBUTES.has(name) || (PER_ELEMENT[tag] ?? []).includes(name);
    if (name === "class") {
      const classes = attribute.value.split(/\s+/).filter((c) => CLASS.test(c));
      if (classes.length) {
        element.setAttribute("class", classes.join(" "));
        continue;
      }
    }
    if (keep && name === "href") keep = safeUrl(attribute.value, SAFE_LINK);
    if (keep && (name === "src" || name === "poster")) keep = safeUrl(attribute.value, SAFE_SOURCE);
    if (keep && name === "srcset") {
      keep = attribute.value.split(",").every((c) => safeUrl(c.trim().split(/\s+/)[0] ?? "", SAFE_SOURCE));
    }
    if (keep && (name === "id" || name === "name") && !attribute.value.startsWith(PREFIX)) {
      // As on GitHub: a README's ids and names cannot shadow the page's own (DOM clobbering).
      element.setAttribute(name, PREFIX + attribute.value);
      continue;
    }
    if (!keep) element.removeAttribute(attribute.name);
  }
  if (element.localName === "input") element.setAttribute("disabled", "");
}

/** Sanitize the children of `root` in place. */
function walk(root) {
  for (const node of [...root.childNodes]) {
    if (node.nodeType === Node.COMMENT_NODE || node.nodeType === Node.PROCESSING_INSTRUCTION_NODE) {
      node.remove();
      continue;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) continue;
    const tag = node.localName;
    const checkbox = tag === "input" && node.getAttribute("type")?.toLowerCase() === "checkbox";
    if (REMOVE.has(tag) || node.namespaceURI !== "http://www.w3.org/1999/xhtml" || (tag === "input" && !checkbox)) {
      node.remove();
      continue;
    }
    walk(node);
    if (ELEMENTS.has(tag)) {
      clean(node);
    } else {
      node.replaceWith(...node.childNodes);
    }
  }
}

/** A sanitized copy of rendered README HTML, as a document body to keep working on. */
export function sanitize(html) {
  // Parsing into an inert document runs no script and loads nothing.
  const doc = new DOMParser().parseFromString(`<!doctype html><body>${html}`, "text/html");
  walk(doc.body);
  return doc;
}
