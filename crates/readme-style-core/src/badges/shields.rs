//! Badge images and their shields.io URLs.

use serde::{Deserialize, Serialize};

/// The shields.io badge style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BadgeStyle {
    Flat,
    FlatSquare,
    ForTheBadge,
    Plastic,
    Social,
}

impl BadgeStyle {
    pub fn id(self) -> &'static str {
        match self {
            BadgeStyle::Flat => "flat",
            BadgeStyle::FlatSquare => "flat-square",
            BadgeStyle::ForTheBadge => "for-the-badge",
            BadgeStyle::Plastic => "plastic",
            BadgeStyle::Social => "social",
        }
    }
}

/// How every badge of a theme looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Look<'a> {
    pub style: BadgeStyle,
    /// Replaces each badge's label (left side) color.
    pub label_color: Option<&'a str>,
    /// Replaces each badge's message (right side) color.
    pub color: Option<&'a str>,
}

/// What a badge shows, independent of style.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Image {
    /// A live shields.io badge, such as `pypi/v/name`.
    Dynamic {
        path: String,
        label: Option<String>,
        logo: Option<&'static str>,
        logo_color: Option<&'static str>,
    },
    /// A fixed label and message.
    Static {
        label: String,
        message: String,
        color: &'static str,
        logo: Option<&'static str>,
        logo_color: Option<&'static str>,
    },
    /// A shields.io endpoint badge reading JSON from `url`.
    Endpoint { url: String },
}

impl Image {
    /// The same image with its logo drawn in `color`.
    #[must_use]
    pub fn with_logo_color(mut self, color: &'static str) -> Self {
        match &mut self {
            Image::Dynamic { logo_color, .. } | Image::Static { logo_color, .. } => *logo_color = Some(color),
            Image::Endpoint { .. } => {}
        }
        self
    }

    pub fn url(&self, look: &Look<'_>) -> String {
        let mut query: Vec<(&str, String)> = Vec::new();
        let path = match self {
            Image::Dynamic {
                path,
                label,
                logo,
                logo_color,
            } => {
                if let Some(label) = label {
                    query.push(("label", label.clone()));
                }
                if let Some(logo) = logo {
                    query.push(("logo", (*logo).to_owned()));
                }
                if let Some(logo_color) = logo_color {
                    query.push(("logoColor", (*logo_color).to_owned()));
                }
                path.clone()
            }
            Image::Static {
                label,
                message,
                color,
                logo,
                logo_color,
            } => {
                if let Some(logo) = logo {
                    query.push(("logo", (*logo).to_owned()));
                }
                if let Some(logo_color) = logo_color {
                    query.push(("logoColor", (*logo_color).to_owned()));
                }
                let color = look.color.unwrap_or(color);
                if label.is_empty() {
                    format!("badge/{}-{color}", segment(message))
                } else {
                    format!("badge/{}-{}-{color}", segment(label), segment(message))
                }
            }
            Image::Endpoint { url } => {
                query.push(("url", url.clone()));
                "endpoint".to_owned()
            }
        };
        if let Some(label_color) = look.label_color {
            query.push(("labelColor", label_color.to_owned()));
        }
        if let (Some(color), false) = (look.color, matches!(self, Image::Static { .. })) {
            query.push(("color", color.to_owned()));
        }
        query.push(("style", look.style.id().to_owned()));
        let query: Vec<String> = query
            .into_iter()
            .map(|(k, v)| format!("{k}={}", encode(&v)))
            .collect();
        format!("https://img.shields.io/{path}?{}", query.join("&"))
    }
}

/// A static badge path segment: shields.io reads `-` and `_` specially.
fn segment(text: &str) -> String {
    encode(&text.replace('-', "--").replace('_', "__").replace(' ', "_"))
}

/// Percent-encode everything but unreserved characters.
fn encode(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAT: Look<'static> = Look {
        style: BadgeStyle::Flat,
        label_color: None,
        color: None,
    };

    #[test]
    fn static_badges_escape_dashes_and_spaces() {
        let image = Image::Static {
            label: "license".into(),
            message: "BSD-3-Clause OR MIT".into(),
            color: "blue",
            logo: None,
            logo_color: None,
        };
        assert_eq!(
            image.url(&FLAT),
            "https://img.shields.io/badge/license-BSD--3--Clause_OR_MIT-blue?style=flat"
        );
    }

    #[test]
    fn dynamic_and_endpoint_badges() {
        let image = Image::Dynamic {
            path: "pypi/v/x".into(),
            label: Some("PyPI version".into()),
            logo: Some("pypi"),
            logo_color: None,
        };
        assert_eq!(
            image.url(&FLAT),
            "https://img.shields.io/pypi/v/x?label=PyPI%20version&logo=pypi&style=flat"
        );
        let image = Image::Endpoint {
            url: "https://e.x/b.json".into(),
        };
        assert_eq!(
            image.url(&FLAT),
            "https://img.shields.io/endpoint?url=https%3A%2F%2Fe.x%2Fb.json&style=flat"
        );
    }

    #[test]
    fn a_palette_recolors_every_badge() {
        let look = Look {
            style: BadgeStyle::FlatSquare,
            label_color: Some("1f2328"),
            color: Some("3f4650"),
        };
        let image = Image::Static {
            label: "a".into(),
            message: "b".into(),
            color: "blue",
            logo: None,
            logo_color: None,
        };
        assert_eq!(
            image.url(&look),
            "https://img.shields.io/badge/a-b-3f4650?labelColor=1f2328&style=flat-square"
        );
        let image = Image::Dynamic {
            path: "npm/v/x".into(),
            label: None,
            logo: None,
            logo_color: None,
        };
        assert_eq!(
            image.url(&look),
            "https://img.shields.io/npm/v/x?labelColor=1f2328&color=3f4650&style=flat-square"
        );
    }
}
