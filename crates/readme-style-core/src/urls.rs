//! Small URL helpers: hosts, GitHub repositories, badge images, relative paths.

use std::sync::LazyLock;

use regex::Regex;
use url::Url;

static GITHUB_REPO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"github\.com[:/]+([\w.-]+)/([\w.-]+?)(?:\.git)?/?(?:[#?].*)?$").expect("valid regex")
});

/// Hosts that serve status badges.
const BADGE_HOSTS: &[&str] = &[
    "img.shields.io",
    "shields.io",
    "badgen.net",
    "badge.fury.io",
    "codecov.io",
    "coveralls.io",
    "travis-ci.org",
    "travis-ci.com",
    "circleci.com",
    "ci.appveyor.com",
    "api.codeclimate.com",
    "app.codacy.com",
    "snyk.io",
    "pepy.tech",
    "static.pepy.tech",
    "goreportcard.com",
    "results.pre-commit.ci",
    "api.securityscorecards.dev",
    "www.bestpractices.dev",
    "deepwiki.com",
    "app.fossa.com",
    "img.badgesize.io",
    "flat.badgen.net",
];

/// The lowercased host of an absolute URL, without a leading `www.`.
pub(crate) fn host(url: &str) -> Option<String> {
    let parsed = Url::parse(url.trim()).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    Some(host.strip_prefix("www.").map(str::to_owned).unwrap_or(host))
}

/// The path of an absolute URL, without leading or trailing slashes.
pub(crate) fn path(url: &str) -> Option<String> {
    let parsed = Url::parse(url.trim()).ok()?;
    Some(parsed.path().trim_matches('/').to_owned())
}

/// Whether an image URL looks like a status badge.
pub(crate) fn is_badge(url: &str) -> bool {
    let url = url.trim();
    if let Ok(parsed) = Url::parse(url) {
        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if BADGE_HOSTS.contains(&host.as_str()) {
            return true;
        }
        let path = parsed.path().to_ascii_lowercase();
        return path.contains("badge") && (is_svg(&path) || path.contains("/badge"));
    }
    url.to_ascii_lowercase().contains("badge") && is_svg(url)
}

fn is_svg(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
}

/// `owner/name` when the URL points at a GitHub repository.
pub(crate) fn github_repo(url: &str) -> Option<String> {
    let captures = GITHUB_REPO.captures(url.trim())?;
    Some(format!("{}/{}", &captures[1], &captures[2]))
}

/// A relative image path in canonical form: no `./`, no `a/../`.
pub(crate) fn normalize_relative(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.trim().split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_repos() {
        assert_eq!(
            github_repo("git@github.com:acme/tool.git").as_deref(),
            Some("acme/tool")
        );
        assert_eq!(
            github_repo("https://github.com/acme/tool").as_deref(),
            Some("acme/tool")
        );
        assert_eq!(
            github_repo("git+https://github.com/acme/tool.git").as_deref(),
            Some("acme/tool")
        );
        assert_eq!(
            github_repo("https://github.com/acme/tool#readme").as_deref(),
            Some("acme/tool")
        );
        assert_eq!(github_repo("https://gitlab.com/acme/tool"), None);
    }

    #[test]
    fn badges() {
        assert!(is_badge("https://img.shields.io/pypi/v/x"));
        assert!(is_badge(
            "https://github.com/o/r/actions/workflows/ci.yml/badge.svg"
        ));
        assert!(is_badge("https://codecov.io/gh/o/r/graph/badge.svg"));
        assert!(!is_badge("https://raw.githubusercontent.com/o/r/main/logo.png"));
        assert!(!is_badge("assets/logo.svg"));
    }

    #[test]
    fn relative_paths() {
        assert_eq!(normalize_relative("./assets//logo.svg"), "assets/logo.svg");
        assert_eq!(normalize_relative("docs/../assets/logo.svg"), "assets/logo.svg");
        assert_eq!(host("https://www.Example.com/a").as_deref(), Some("example.com"));
    }
}
