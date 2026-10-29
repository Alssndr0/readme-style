//! Line endings and byte order marks: normalized on the way in, restored on the way out.

/// A README's text with LF line endings and no BOM, remembering how to restore both.
#[derive(Debug, Clone)]
pub(crate) struct Normalized {
    pub(crate) text: String,
    bom: bool,
    crlf: bool,
}

impl Normalized {
    pub(crate) fn new(raw: &str) -> Self {
        let bom = raw.starts_with('\u{feff}');
        let body = raw.strip_prefix('\u{feff}').unwrap_or(raw);
        let crlf = body.contains("\r\n");
        let text = if crlf {
            body.replace("\r\n", "\n")
        } else {
            body.to_owned()
        };
        Self { text, bom, crlf }
    }

    pub(crate) fn restore(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len() + text.len() / 32 + 3);
        if self.bom {
            out.push('\u{feff}');
        }
        if self.crlf {
            out.push_str(&text.replace('\n', "\r\n"));
        } else {
            out.push_str(text);
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
    }

    #[test]
    fn leaves_plain_text_alone() {
        let normalized = Normalized::new("a\nb");
        assert_eq!(normalized.restore("a\nb\nc"), "a\nb\nc");
    }
}
