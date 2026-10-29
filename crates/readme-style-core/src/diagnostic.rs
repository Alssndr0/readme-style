//! Non-fatal findings reported alongside a result.

use std::fmt;

use serde::Serialize;

/// A problem worth telling the user about that did not stop styling, such as a marker
/// that is never closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// 1-based line in the README.
    pub line: usize,
    /// What is wrong, in a sentence.
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}
