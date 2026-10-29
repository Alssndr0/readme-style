//! License detection from license file text.

use std::sync::LazyLock;

use regex::Regex;

static TEXTS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    [
        (r"GNU AFFERO GENERAL PUBLIC LICENSE", "AGPL-3.0"),
        (r"GNU LESSER GENERAL PUBLIC LICENSE\s+Version 2\.1", "LGPL-2.1"),
        (r"GNU LESSER GENERAL PUBLIC LICENSE", "LGPL-3.0"),
        (r"GNU GENERAL PUBLIC LICENSE\s+Version 2", "GPL-2.0"),
        (r"GNU GENERAL PUBLIC LICENSE", "GPL-3.0"),
        (r"Apache License,?\s+Version 2\.0", "Apache-2.0"),
        (r"Mozilla Public License,? (?:Version )?2\.0", "MPL-2.0"),
        (r"Boost Software License", "BSL-1.0"),
        (
            r"This is free and unencumbered software released into the public domain",
            "Unlicense",
        ),
        (r"Creative Commons Attribution 4\.0", "CC-BY-4.0"),
        (r"CC0 1\.0 Universal", "CC0-1.0"),
        (
            r"Permission to use, copy, modify, and/or distribute this software for any",
            "ISC",
        ),
        (r"Permission is hereby granted, free of charge", "MIT"),
    ]
    .into_iter()
    .map(|(pattern, spdx)| (Regex::new(&format!("(?i){pattern}")).expect("valid regex"), spdx))
    .collect()
});

/// The SPDX identifier of a license file's text, when it is a well-known license.
pub fn detect_license(text: &str) -> Option<&'static str> {
    if let Some((_, spdx)) = TEXTS.iter().find(|(pattern, _)| pattern.is_match(text)) {
        return Some(spdx);
    }
    let lower = text.to_ascii_lowercase();
    if lower.contains("redistribution and use in source and binary forms") {
        return Some(if lower.contains("neither the name") {
            "BSD-3-Clause"
        } else {
            "BSD-2-Clause"
        });
    }
    None
}

/// Map a trove classifier's license name to SPDX.
pub(super) fn from_classifier(name: &str) -> Option<&'static str> {
    Some(match name {
        "MIT License" => "MIT",
        "Apache Software License" => "Apache-2.0",
        "BSD License" => "BSD",
        "ISC License (ISCL)" => "ISC",
        "GNU General Public License v3 (GPLv3)" => "GPL-3.0",
        "GNU General Public License v2 (GPLv2)" => "GPL-2.0",
        "GNU Lesser General Public License v3 (LGPLv3)" => "LGPL-3.0",
        "GNU Affero General Public License v3" => "AGPL-3.0",
        "Mozilla Public License 2.0 (MPL 2.0)" => "MPL-2.0",
        "The Unlicense (Unlicense)" => "Unlicense",
        _ => return None,
    })
}

/// A short license string from metadata, or `None` for license texts pasted into metadata.
pub(super) fn spdx(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.len() <= 60 && !value.contains('\n')).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::detect_license;

    #[test]
    fn detects_common_licenses() {
        assert_eq!(
            detect_license("Apache License\n Version 2.0, January 2004"),
            Some("Apache-2.0")
        );
        assert_eq!(
            detect_license("MIT License\n\nPermission is hereby granted, free of charge"),
            Some("MIT")
        );
        assert_eq!(
            detect_license("Redistribution and use in source and binary forms ... Neither the name"),
            Some("BSD-3-Clause")
        );
        assert_eq!(
            detect_license("Redistribution and use in source and binary forms"),
            Some("BSD-2-Clause")
        );
        assert_eq!(detect_license("All rights reserved."), None);
    }
}
