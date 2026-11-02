//! craft-web-overrides' manifest.json, release tags and version names.
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Override {
    /// folder name under versions/ (`<version>-z<id>`)
    pub dir: String,
    pub url: String,
    pub sha256: String,
    /// release notes page ("What's patched" link)
    pub release: String,
}

/// `app -> entry` for every well-formed entry; anything else in the JSON is ignored.
pub fn parse_overrides(json: &[u8]) -> BTreeMap<String, Override> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_slice(json) else {
        return BTreeMap::new();
    };
    map.iter()
        .filter_map(|(app, e)| {
            let s = |k: &str| e.get(k)?.as_str().map(str::to_owned);
            Some((
                app.clone(),
                Override { dir: s("dir")?, url: s("url")?, sha256: s("sha256")?, release: s("release")? },
            ))
        })
        .collect()
}

pub fn is_json(bytes: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(bytes).is_ok()
}

/// `sed 's|.*/tag/||'` on the releases/latest redirect: everything after the last `/tag/`.
pub fn tag_from_location(loc: &str) -> &str {
    loc.rfind("/tag/").map_or(loc, |i| &loc[i + 5..])
}

/// The version names a directory that gets replaced, so it has to be a plain name:
/// `[0-9A-Za-z][0-9A-Za-z.+_-]*`, no `..`.
pub fn is_plain_version(v: &str) -> bool {
    let mut cs = v.chars();
    cs.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && cs.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '_' | '-'))
        && !v.contains("..")
}

/// An app name ends up in paths and URLs.
pub fn is_plain_app(a: &str) -> bool {
    !a.is_empty() && a.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Override zips come from the project's own releases and nowhere else.
pub fn is_trusted_override_url(u: &str) -> bool {
    u.starts_with("https://github.com/zuujiscom/craft-web-overrides/releases/download/")
}

/// `${v%-z*}`: the version shown on the page, without the patch id.
pub fn display_version(v: &str) -> &str {
    v.rfind("-z").map_or(v, |i| &v[..i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_versions() {
        for ok in ["0.16.0", "0.16.0-zc7f0e87b", "1", "v1.2.3+build_4", "0.9.0-rc.1"] {
            assert!(is_plain_version(ok), "{ok}");
        }
        for bad in ["", "-1", ".1", "1..2", "../x", "a/b", "1 2", "a\n", "1;rm", "é"] {
            assert!(!is_plain_version(bad), "{bad}");
        }
    }

    #[test]
    fn tags() {
        assert_eq!(tag_from_location("https://github.com/storytold/photocraft/releases/tag/v0.11.1"), "v0.11.1");
        assert_eq!(tag_from_location("https://x/tag/a/tag/b"), "b");
        // no /tag/: whole text stays and fails the version check
        assert!(!is_plain_version(tag_from_location("https://github.com/login")));
        assert_eq!("v0.11.1".strip_prefix('v'), Some("0.11.1"));
    }

    #[test]
    fn display() {
        assert_eq!(display_version("0.16.0-zc7f0e87b"), "0.16.0");
        assert_eq!(display_version("0.16.0"), "0.16.0");
        assert_eq!(display_version("1-za-zb"), "1-za");
    }

    #[test]
    fn manifest() {
        let j = br#"{"cadcraft":{"dir":"0.16.0-zc7f0e87b","id":"c7f0e87b","patches":[],"release":"https://r/1","sha256":"ab","url":"https://u/1.zip","version":"0.16.0"},
                     "broken":{"dir":"1"}, "num":5}"#;
        let m = parse_overrides(j);
        assert_eq!(m.len(), 1);
        let c = &m["cadcraft"];
        assert_eq!(
            (c.dir.as_str(), c.url.as_str(), c.sha256.as_str(), c.release.as_str()),
            ("0.16.0-zc7f0e87b", "https://u/1.zip", "ab", "https://r/1")
        );
        assert!(parse_overrides(b"[1]").is_empty());
        assert!(parse_overrides(b"not json").is_empty());
        assert!(is_json(b"{}") && !is_json(b"<html>"));
    }

    #[test]
    fn real_manifest_parses() {
        let m = parse_overrides(include_bytes!("../../../manifest.json"));
        assert!(!m.is_empty());
        for (app, e) in &m {
            assert!(is_plain_app(app) && is_plain_version(&e.dir) && is_trusted_override_url(&e.url), "{app}");
            assert_eq!(e.sha256.len(), 64);
        }
    }

    #[test]
    fn urls_and_apps() {
        assert!(is_trusted_override_url("https://github.com/zuujiscom/craft-web-overrides/releases/download/a/b.zip"));
        assert!(!is_trusted_override_url("http://github.com/zuujiscom/craft-web-overrides/releases/download/a"));
        assert!(!is_trusted_override_url("https://evil.example/zuujiscom/craft-web-overrides/releases/download/"));
        assert!(is_plain_app("photocraft") && !is_plain_app("../x") && !is_plain_app(""));
    }
}
