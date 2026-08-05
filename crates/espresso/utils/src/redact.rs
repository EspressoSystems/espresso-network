//! Redaction of credentials embedded in provider URLs. The host is kept so logs still identify
//! which provider failed; a credential in the hostname is not redactable.
//!
//! [`scrub`] takes rendered text and handles the two forms a URL appears in, which share no common
//! substring: `https://host/v2/KEY` from `reqwest` and friends, and `url::Url`'s own field-wise
//! `Debug`, which contains no `://` at all. [`redact_url`] takes a `Url` directly.
use std::fmt;

use url::Url;

const REDACTED: &str = "***";

/// Characters `Url::as_str()` percent-encodes in every component, so they cannot occur inside a
/// rendered URL. Nothing may be added here without that guarantee: `)`, `,`, `|`, `^`, `` ` ``, `{`
/// and `}` are legal in a path, and ending the token on one leaks the tail after it.
fn ends_url_token(c: char) -> bool {
    c == ' ' || c.is_control() || matches!(c, '"' | '<' | '>')
}

fn ends_authority(c: char) -> bool {
    matches!(c, '/' | '?' | '#') || ends_url_token(c)
}

/// Drops the credential-bearing suffix of every URL in `text`. Over-deletes at an ambiguous token
/// boundary rather than emit a partial credential.
pub fn scrub_urls(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;

    while let Some(sep_rel) = text[pos..].find("://") {
        // "://" is ASCII, so +3 lands on a char boundary.
        let auth_start = pos + sep_rel + 3;
        out.push_str(&text[pos..auth_start]);

        let auth_end = text[auth_start..]
            .find(ends_authority)
            .map(|k| auth_start + k)
            .unwrap_or(text.len());
        // Userinfo is a credential: `WsConnect` lifts `wss://user:pass@host` into an Authorization
        // header. Keep only what follows the last `@`.
        match text[auth_start..auth_end].rsplit_once('@') {
            Some((_, host)) => {
                out.push_str(REDACTED);
                out.push('@');
                out.push_str(host);
            },
            None => out.push_str(&text[auth_start..auth_end]),
        }

        let tail_end = text[auth_end..]
            .find(ends_url_token)
            .map(|k| auth_end + k)
            .unwrap_or(text.len());
        if tail_end > auth_end {
            out.push('/');
            out.push_str(REDACTED);
        }
        pos = tail_end;
    }

    out.push_str(&text[pos..]);
    out
}

pub fn redact_url(url: &Url) -> String {
    let scheme = url.scheme();
    let Some(host) = url.host() else {
        return format!("{scheme}:{REDACTED}");
    };
    let userinfo = match url.username().is_empty() && url.password().is_none() {
        true => String::new(),
        false => format!("{REDACTED}@"),
    };
    let port = match url.port() {
        Some(port) => format!(":{port}"),
        None => String::new(),
    };
    let has_tail =
        !matches!(url.path(), "" | "/") || url.query().is_some() || url.fragment().is_some();
    let tail = match has_tail {
        true => format!("/{REDACTED}"),
        false => String::new(),
    };
    format!("{scheme}://{userinfo}{host}{port}{tail}")
}

pub fn redact_urls<'a>(urls: impl IntoIterator<Item = &'a Url>) -> Vec<String> {
    urls.into_iter().map(redact_url).collect()
}

const URL_DEBUG_PREFIX: &str = "Url {";

/// Collapses the body of every `url::Url` field-wise `Debug` in `text`, host included. Reaching
/// this means a struct failed to redact a `Url` field, so `Url { *** }` is a signal to fix that
/// struct rather than output to read a host out of.
pub fn scrub_url_debug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;

    while let Some(rel) = text[pos..].find(URL_DEBUG_PREFIX) {
        let body_start = pos + rel + URL_DEBUG_PREFIX.len();
        out.push_str(&text[pos..body_start]);
        // `Url`'s Debug body contains no nested braces, so the next `}` closes it.
        let Some(k) = text[body_start..].find('}') else {
            // Truncated: drop the remainder rather than emit an unterminated body.
            pos = text.len();
            break;
        };
        out.push_str(" *** ");
        pos = body_start + k;
    }

    out.push_str(&text[pos..]);
    out
}

pub fn scrub(text: &str) -> String {
    scrub_url_debug(&scrub_urls(text))
}

/// Wraps a value so its `Debug`/`Display` output passes through [`scrub_urls`].
///
/// Forwards `{:#}`/`{:#?}`, so an `anyhow` chain rendered with `{:#}` keeps every context level.
pub struct Redacted<T>(pub T);

impl<T: fmt::Debug> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = if f.alternate() {
            format!("{:#?}", self.0)
        } else {
            format!("{:?}", self.0)
        };
        f.write_str(&scrub_urls(&rendered))
    }
}

impl<T: fmt::Display> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = if f.alternate() {
            format!("{:#}", self.0)
        } else {
            format!("{}", self.0)
        };
        f.write_str(&scrub_urls(&rendered))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_scrub_urls() {
        let cases: &[(&str, &str)] = &[
            // reqwest's Debug.
            (
                r#"Transport(Custom(reqwest::Error { kind: Request, url: "https://rpc.example.com/v1/FAKEKEY", source: hyper::Error(IncompleteMessage) }))"#,
                r#"Transport(Custom(reqwest::Error { kind: Request, url: "https://rpc.example.com/***", source: hyper::Error(IncompleteMessage) }))"#,
            ),
            // reqwest's Display. The closing paren is absorbed by the fail-closed boundary.
            (
                "error sending request for url (https://host/v1/FAKEKEY)",
                "error sending request for url (https://host/***",
            ),
            ("https://user:pass@host/x", "https://***@host/***"),
            ("https://user@host/x", "https://***@host/***"),
            ("https://user:pass@host@weird/x", "https://***@weird/***"),
            ("https://host/v2/FAKEKEY", "https://host/***"),
            ("https://host/rpc?apikey=FAKEKEY", "https://host/***"),
            ("https://host/p#frag", "https://host/***"),
            ("http://host:8545/k", "http://host:8545/***"),
            ("wss://user:pass@host/v2/FAKEKEY", "wss://***@host/***"),
            ("ws://host:8546/FAKEKEY", "ws://host:8546/***"),
            ("https://[::1]:8545/FAKEKEY", "https://[::1]:8545/***"),
            (
                "https://127.0.0.1:8545/FAKEKEY",
                "https://127.0.0.1:8545/***",
            ),
            (
                r#"url: "https://host/FAKEKEY""#,
                r#"url: "https://host/***""#,
            ),
            // Characters `Url::as_str()` leaves unencoded in a path or query must not terminate
            // the token, or the tail after them survives.
            ("https://host/a)b,FAKEKEY suffix", "https://host/*** suffix"),
            ("https://host/a|FAKEKEY", "https://host/***"),
            ("https://host/a^FAKEKEY", "https://host/***"),
            ("https://host/a`FAKEKEY", "https://host/***"),
            ("https://host/a\\FAKEKEY", "https://host/***"),
            ("https://host/rpc?token={FAKEKEY}", "https://host/***"),
            (
                "url=https://u:p@host/x|FAKEKEY more",
                "url=https://***@host/*** more",
            ),
            (
                "a https://h1/k1 b https://h2/k2 c",
                "a https://h1/*** b https://h2/*** c",
            ),
            (
                "日本 https://host/FAKEKEY 中文",
                "日本 https://host/*** 中文",
            ),
            ("http://localhost:8545", "http://localhost:8545"),
            ("see https://host", "see https://host"),
            ("no url here", "no url here"),
            ("://", "://"),
            ("", ""),
        ];
        for (input, expected) in cases {
            assert_eq!(&scrub_urls(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn test_scrub_urls_idempotent() {
        for input in [
            r#"reqwest::Error { url: "https://rpc.example.com/v1/FAKEKEY" }"#,
            "a https://h1/k1 b https://h2/k2 c",
            "https://user:pass@host@weird/x",
            "日本 https://host/FAKEKEY 中文",
            "http://localhost:8545",
        ] {
            let once = scrub_urls(input);
            assert_eq!(scrub_urls(&once), once, "input: {input:?}");
        }
    }

    #[test]
    fn test_redact_url() {
        let cases: &[(&str, &str)] = &[
            (
                "https://host.example.com/v1/FAKEKEY",
                "https://host.example.com/***",
            ),
            ("http://host:8545/k", "http://host:8545/***"),
            ("https://user:pass@host/x", "https://***@host/***"),
            ("https://host/rpc?apikey=FAKEKEY", "https://host/***"),
            ("https://[::1]:8545/FAKEKEY", "https://[::1]:8545/***"),
            ("ws://host:8546/FAKEKEY", "ws://host:8546/***"),
            ("http://localhost:8545", "http://localhost:8545"),
            ("mailto:foo@example.com", "mailto:***"),
        ];
        for (input, expected) in cases {
            let url = Url::parse(input).unwrap();
            assert_eq!(&redact_url(&url), expected, "input: {input:?}");
        }
    }

    /// `scrub_urls` cannot see the field-wise form, which is why `scrub_url_debug` exists.
    #[test]
    fn test_scrub_urls_misses_url_debug() {
        let raw = format!(
            "{:?}",
            Url::parse("https://user:pass@host.example.com/v2/FAKEKEY").unwrap()
        );
        assert!(raw.contains("FAKEKEY"), "{raw}");
        assert!(!raw.contains("://"), "{raw}");
        assert_eq!(scrub_urls(&raw), raw);
    }

    #[test]
    fn test_scrub_url_debug() {
        let raw = format!(
            "urls: [{:?}]",
            Url::parse("https://user:pass@host.example.com/v2/FAKEKEY").unwrap()
        );
        let scrubbed = scrub_url_debug(&raw);
        assert_eq!(scrubbed, "urls: [Url { *** }]");
        assert!(!scrubbed.contains("FAKEKEY"));
        assert!(!scrubbed.contains("user"));
        assert_eq!(scrub_url_debug(&scrubbed), scrubbed);
    }

    #[test]
    fn test_scrub_url_debug_multiple_and_unterminated() {
        let two = format!(
            "[{:?}, {:?}]",
            Url::parse("https://a.test/K1").unwrap(),
            Url::parse("wss://b.test/K2").unwrap()
        );
        let scrubbed = scrub_url_debug(&two);
        assert_eq!(scrubbed, "[Url { *** }, Url { *** }]");

        // Truncated debug output must not emit the partial body.
        let cut = r#"urls: [Url { scheme: "https", path: "/v2/FAKEKEY""#;
        assert!(!scrub_url_debug(cut).contains("FAKEKEY"));
    }

    #[test]
    fn test_scrub_handles_both_forms() {
        let mixed = format!(
            "reqwest url: \"https://h.test/v1/FAKEKEY\" and {:?}",
            Url::parse("https://u:p@h2.test/v2/OTHERKEY").unwrap()
        );
        let scrubbed = scrub(&mixed);
        assert!(!scrubbed.contains("FAKEKEY"), "{scrubbed}");
        assert!(!scrubbed.contains("OTHERKEY"), "{scrubbed}");
        assert!(scrubbed.contains("https://h.test/***"), "{scrubbed}");
        assert_eq!(scrub(&scrubbed), scrubbed);
    }

    #[test]
    fn test_redacted_scrubs_debug_and_display() {
        let value = "error at https://host.example.com/v2/FAKEKEY";
        assert_eq!(
            format!("{:?}", Redacted(value)),
            r#""error at https://host.example.com/***""#
        );
        assert_eq!(
            format!("{}", Redacted(value)),
            "error at https://host.example.com/***"
        );
        assert_eq!(
            format!("{:#?}", Redacted(value)),
            r#""error at https://host.example.com/***""#
        );
    }

    #[test]
    fn test_redacted_alternate_keeps_anyhow_chain() {
        let err = anyhow::Error::msg("https://host.example.com/v2/FAKEKEY unreachable")
            .context("fetching L1 head")
            .context("initializing client");
        let redacted = format!("{:#}", Redacted(&err));
        assert!(redacted.contains("initializing client"), "{redacted}");
        assert!(redacted.contains("fetching L1 head"), "{redacted}");
        assert!(
            redacted.contains("https://host.example.com/***"),
            "{redacted}"
        );
        assert!(!redacted.contains("FAKEKEY"), "{redacted}");
    }
}
