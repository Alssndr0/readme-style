//! The engine behind [readme-style](https://github.com/Alssndr0/readme-style): restyle a
//! README's presentation without changing what it says.
//!
//! ```
//! use readme_style_core::{style, project::ProjectFacts, theme::Themes};
//!
//! let theme = Themes::builtin().get("minimal").unwrap();
//! let facts = ProjectFacts { license: Some("MIT".into()), ..ProjectFacts::default() };
//! let outcome = style("# demo\n\n> **Note:** hello\n", &theme, &facts).unwrap();
//! assert!(outcome.output.contains("> [!NOTE]"));
//! assert!(outcome.output.contains("license-MIT"));
//! ```
//!
//! [`style`] never returns a README that says something different from its input: see
//! [`guard`]. Project facts come from [`project::inspect`], themes from [`theme::Themes`],
//! user configuration from [`config::discover`].

pub mod badges;
pub mod config;
mod diagnostic;
mod document;
mod edit;
mod engine;
pub mod guard;
mod html;
mod markdown;
mod markers;
mod outline;
pub mod project;
mod slug;
mod source;
pub mod theme;
mod transform;
mod urls;

pub use diagnostic::Diagnostic;
pub use engine::{Outcome, StyleError, recorded_theme, style, style_unchecked};

/// Render Markdown to HTML as readme-style reads it (comrak with GitHub's extensions), with an
/// `id` on every heading so links to sections work, for previews.
///
/// GitHub also removes scripts, event handlers and other unsafe HTML; this does not. Show the
/// result only where its scripts cannot run, such as a sandboxed frame.
pub fn render_html(markdown: &str) -> String {
    markdown::to_html_for_preview(markdown)
}
