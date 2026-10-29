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
