//! readme-style for the browser: the engine compiled to WebAssembly behind a small interface
//! that needs no JavaScript glue library.
//!
//! JavaScript asks [`input`] for a buffer, writes a request into it as UTF-8, calls [`style`],
//! [`render`] or [`themes`], and reads the answer from [`output`] for as many bytes as the call
//! returned. Nothing here uses the network or a file system: the project's files come in the
//! request and are read from memory.
//!
//! A panic traps the module; [`panic_message`] then puts the panic's message in the output
//! buffer. JavaScript should start a fresh instance after a trap.

pub mod api;

use std::cell::RefCell;

thread_local! {
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static PANIC: RefCell<String> = const { RefCell::new(String::new()) };
}

/// A buffer of `len` bytes for the next request, which JavaScript fills.
#[allow(unsafe_code)]
#[cfg_attr(target_family = "wasm", unsafe(no_mangle))]
pub extern "C" fn input(len: usize) -> *mut u8 {
    INPUT.with_borrow_mut(|buffer| {
        buffer.clear();
        buffer.resize(len, 0);
        buffer.as_mut_ptr()
    })
}

/// Where the last answer starts.
#[allow(unsafe_code)]
#[cfg_attr(target_family = "wasm", unsafe(no_mangle))]
pub extern "C" fn output() -> *const u8 {
    OUTPUT.with_borrow(Vec::as_ptr)
}

/// Style the README in the request, a JSON [`api::StyleRequest`]. Answers a JSON
/// [`api::StyleResponse`]; returns its length.
#[allow(unsafe_code)]
#[cfg_attr(target_family = "wasm", unsafe(no_mangle))]
pub extern "C" fn style() -> usize {
    respond(|request| {
        let response = match serde_json::from_str::<api::StyleRequest>(request) {
            Ok(request) => api::style_readme(&request),
            Err(error) => api::StyleResponse::error(format!("invalid request: {error}"), Vec::new()),
        };
        serde_json::to_string(&response).unwrap_or_default()
    })
}

/// Render the Markdown in the request to HTML (see [`readme_style_core::render_html`]); returns
/// the length of the HTML.
#[allow(unsafe_code)]
#[cfg_attr(target_family = "wasm", unsafe(no_mangle))]
pub extern "C" fn render() -> usize {
    respond(readme_style_core::render_html)
}

/// The built-in themes as JSON, ignoring the request; returns the length of the answer.
#[allow(unsafe_code)]
#[cfg_attr(target_family = "wasm", unsafe(no_mangle))]
pub extern "C" fn themes() -> usize {
    respond(|_| serde_json::to_string(&api::themes()).unwrap_or_default())
}

/// The message of the panic that trapped the module, as the answer; returns its length.
#[allow(unsafe_code)]
#[cfg_attr(target_family = "wasm", unsafe(no_mangle))]
pub extern "C" fn panic_message() -> usize {
    let message = PANIC.with(|p| p.try_borrow().map(|m| m.clone()).unwrap_or_default());
    answer(message)
}

fn respond(handler: impl FnOnce(&str) -> String) -> usize {
    // Only in the browser: natively, the hook would hide the messages of failing tests.
    #[cfg(target_family = "wasm")]
    {
        static HOOK: std::sync::Once = std::sync::Once::new();
        HOOK.call_once(|| {
            std::panic::set_hook(Box::new(|info| {
                PANIC.with(|p| {
                    if let Ok(mut message) = p.try_borrow_mut() {
                        *message = info.to_string();
                    }
                });
            }));
        });
    }
    let request = INPUT.with_borrow(|bytes| String::from_utf8_lossy(bytes).into_owned());
    answer(handler(&request))
}

fn answer(text: String) -> usize {
    OUTPUT.with_borrow_mut(|buffer| {
        *buffer = text.into_bytes();
        buffer.len()
    })
}

#[cfg(test)]
mod tests {
    use super::{input, output, render, style};

    fn call(request: &str, function: extern "C" fn() -> usize) -> String {
        let pointer = input(request.len());
        // Natively the buffer is ordinary memory; fill it the way JavaScript would.
        super::INPUT.with_borrow_mut(|buffer| {
            assert_eq!(buffer.as_mut_ptr(), pointer);
            buffer.copy_from_slice(request.as_bytes());
        });
        let len = function();
        assert!(!output().is_null());
        super::OUTPUT.with_borrow(|bytes| {
            assert_eq!(bytes.len(), len);
            String::from_utf8(bytes.clone()).expect("UTF-8")
        })
    }

    #[test]
    fn requests_and_answers_go_through_the_buffers() {
        let answer = call(r##"{"readme": "# demo\n\n> **Note:** hi\n"}"##, style);
        let answer: serde_json::Value = serde_json::from_str(&answer).expect("JSON");
        assert_eq!(answer["status"], "changed");
        assert_eq!(answer["theme"], "minimal");
        let html = call("# Usage\n", render);
        assert!(html.contains(r#"id="usage""#), "{html}");
        let invalid = call("not json", style);
        assert!(invalid.contains("invalid request"), "{invalid}");
    }
}
