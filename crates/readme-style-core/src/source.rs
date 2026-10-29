//! Line endings and byte order marks: normalized on the way in, restored on the way out.
//!
//! The engine works on text with `\n` line endings. Every original line keeps its own ending
//! (`\n`, `\r\n`, or a lone `\r`, which `CommonMark` also treats as a line ending): lines the
//! styling did not touch get theirs back, and new lines get the file's most common one. A file
//! with a single pasted CRLF line therefore keeps exactly that line as it was.

use similar::{Algorithm, DiffOp, capture_diff_slices};

const BOM: char = '\u{feff}';

#[derive(Debug, Clone)]
pub(crate) struct Normalized {
    /// The text with `\n` line endings and no byte order mark.
    pub(crate) text: String,
    raw: String,
    bom: bool,
    /// Each original line and the ending it had ("" for a last line without one).
    lines: Vec<(String, &'static str)>,
    /// The ending for new lines.
    newline: &'static str,
}

fn split(text: &str) -> Vec<(String, &'static str)> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' if chars.peek() == Some(&'\n') => {
                chars.next();
                lines.push((std::mem::take(&mut current), "\r\n"));
            }
            '\r' => lines.push((std::mem::take(&mut current), "\r")),
            '\n' => lines.push((std::mem::take(&mut current), "\n")),
            other => current.push(other),
        }
    }
    if !current.is_empty() {
        lines.push((current, ""));
    }
    lines
}

impl Normalized {
    pub(crate) fn new(raw: &str) -> Self {
        let bom = raw.starts_with(BOM);
        let body = raw.strip_prefix(BOM).unwrap_or(raw);
        let lines = split(body);
        let count = |ending: &str| lines.iter().filter(|(_, e)| *e == ending).count();
        let newline = if count("\r\n") > count("\n") { "\r\n" } else { "\n" };
        let mut text = String::with_capacity(body.len());
        for (line, ending) in &lines {
            text.push_str(line);
            if !ending.is_empty() {
                text.push('\n');
            }
        }
        Self {
            text,
            raw: raw.to_owned(),
            bom,
            lines,
            newline,
        }
    }

    /// `text` (a styled version of [`Self::text`]) with the original endings and BOM.
    pub(crate) fn restore(&self, text: &str) -> String {
        if text == self.text {
            return self.raw.clone();
        }
        let ends_with_newline = text.ends_with('\n');
        let new: Vec<&str> = text.strip_suffix('\n').unwrap_or(text).split('\n').collect();
        let old: Vec<&str> = self.lines.iter().map(|(line, _)| line.as_str()).collect();
        let mut endings: Vec<&'static str> = vec![self.newline; new.len()];
        for op in capture_diff_slices(Algorithm::Myers, &old, &new) {
            if let DiffOp::Equal {
                old_index,
                new_index,
                len,
            } = op
            {
                for k in 0..len {
                    let ending = self.lines[old_index + k].1;
                    endings[new_index + k] = if ending.is_empty() { self.newline } else { ending };
                }
            }
        }
        let mut out = String::with_capacity(text.len() + 16);
        if self.bom {
            out.push(BOM);
        }
        for (i, line) in new.iter().enumerate() {
            out.push_str(line);
            if i + 1 < new.len() || ends_with_newline {
                out.push_str(endings[i]);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Normalized;

    #[test]
    fn round_trips_bom_and_crlf() {
        let raw = "\u{feff}# Title\r\n\r\ntext\r\n";
        let normalized = Normalized::new(raw);
        assert_eq!(normalized.text, "# Title\n\ntext\n");
        assert_eq!(normalized.restore(&normalized.text), raw);
        assert_eq!(
            normalized.restore("# Title\n\nnew\n\ntext\n"),
            "\u{feff}# Title\r\n\r\nnew\r\n\r\ntext\r\n"
        );
    }

    #[test]
    fn each_line_keeps_its_ending() {
        let raw = "# T\n\nmixed\r\nprogress 10%\rprogress 100%\nend";
        let normalized = Normalized::new(raw);
        assert_eq!(normalized.text, "# T\n\nmixed\nprogress 10%\nprogress 100%\nend");
        let styled = "# T\n\nadded\n\nmixed\nprogress 10%\nprogress 100%\nend";
        assert_eq!(
            normalized.restore(styled),
            "# T\n\nadded\n\nmixed\r\nprogress 10%\rprogress 100%\nend"
        );
    }

    #[test]
    fn a_new_last_line_after_an_unterminated_one() {
        let normalized = Normalized::new("a\nb");
        assert_eq!(normalized.restore("a\nb\nc"), "a\nb\nc");
    }
}
