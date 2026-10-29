// The readme-style engine, compiled to WebAssembly (crates/readme-style-wasm). Requests and
// answers pass through the module's memory as UTF-8; no glue library is needed.

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export class Engine {
  static async load(url) {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`cannot load ${url} (${response.status})`);
    }
    return new Engine(await WebAssembly.compile(await response.arrayBuffer()));
  }

  #module;
  #instance = null;

  constructor(module) {
    this.#module = module;
  }

  /** Style a README: see `StyleRequest` and `StyleResponse` in crates/readme-style-wasm. */
  style(request) {
    return JSON.parse(this.#call("style", JSON.stringify(request)));
  }

  /** Markdown to HTML, as readme-style reads it, with an id on every heading. */
  render(markdown) {
    return this.#call("render", markdown);
  }

  /** The built-in themes: `[{name, description}]`. */
  themes() {
    return JSON.parse(this.#call("themes", ""));
  }

  #exports() {
    this.#instance ??= new WebAssembly.Instance(this.#module, {});
    return this.#instance.exports;
  }

  #call(name, text) {
    const exports = this.#exports();
    const bytes = encoder.encode(text);
    try {
      const pointer = exports.input(bytes.length);
      new Uint8Array(exports.memory.buffer, pointer, bytes.length).set(bytes);
      return this.#read(exports, exports[name]());
    } catch (error) {
      // A panic traps the module. Read its message, then start again with a fresh instance.
      let message = String(error);
      try {
        message = this.#read(exports, exports.panic_message()) || message;
      } catch {
        // The message went down with the instance.
      }
      this.#instance = null;
      throw new Error(`readme-style crashed (${message}). Please report this, with the README.`);
    }
  }

  #read(exports, length) {
    // Memory may have grown during the call: take a fresh view of it.
    return decoder.decode(new Uint8Array(exports.memory.buffer, exports.output(), length));
  }
}
