//! Redaction of credentials embedded in provider URLs. L1 RPC providers put the API key in the
//! path or query (`https://host/v2/KEY`), so the host is kept and everything after it dropped. A
//! credential in the hostname is not redactable.
//!
//! [`scrub`] works on already-rendered text, which is the only handle available when the URL is
//! baked into someone else's `Debug`/`Display` (`reqwest` prints the full request URL in both).
//! [`redact_url`] takes a [`Url`] directly.
//!
//! It handles two shapes that share no common substring. `Url`'s own `Debug` is field-wise, with the
//! host and `path: "/v2/KEY"` as separate fields and no `://` between them, so URL-shaped matching
//! misses it entirely. Structs holding a [`Url`] redact that field themselves; [`scrub_url_debug`]
//! is the net for one that does not.
use url::Url;

const REDACTED: &str = "***";

/// Characters `Url::as_str()` percent-encodes in every component, so they cannot occur inside a
/// rendered URL. Nothing may be added here without that guarantee: `)`, `,`, `|` and `^` are legal
/// in a path, and `` ` ``, `{` and `}` are legal in a query or fragment; ending a URL on any of them
/// would leave the tail after it in the output.
fn ends_url_token(c: char) -> bool {
    c == ' ' || c.is_control() || matches!(c, '"' | '<' | '>')
}

/// Replaces everything after the host of every URL in `text` with `***`.
///
/// Assumes the text was rendered from a parsed [`Url`], which percent-encodes the boundary
/// characters above. Credential-bearing URLs satisfy this by entering as clap-parsed [`Url`]s, so a
/// malformed one is rejected at startup rather than reaching a log line.
fn scrub_urls(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;

    while let Some(rel) = text[pos..].find("://") {
        // "://" is ASCII, so +3 lands on a char boundary.
        let start = pos + rel + 3;
        out.push_str(&text[pos..start]);

        let end = text[start..]
            .find(ends_url_token)
            .map(|k| start + k)
            .unwrap_or(text.len());
        let token = &text[start..end];
        let authority = token.split(['/', '?', '#']).next().unwrap_or(token);
        // Userinfo precedes the host and can carry a password.
        let host = authority.rsplit('@').next().unwrap_or(authority);

        out.push_str(host);
        if host.len() != token.len() {
            out.push('/');
            out.push_str(REDACTED);
        }
        pos = end;
    }

    out.push_str(&text[pos..]);
    out
}

const URL_DEBUG_PREFIX: &str = "Url {";

/// Collapses the body of every `url::Url` field-wise `Debug` in `text`, host included. Reaching
/// this means a struct failed to redact a `Url` field, so `Url { *** }` is a signal to fix that
/// struct rather than output to read a host out of.
fn scrub_url_debug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;

    while let Some(rel) = text[pos..].find(URL_DEBUG_PREFIX) {
        let start = pos + rel;
        let body_start = start + URL_DEBUG_PREFIX.len();
        // Require a type-name boundary, so `BaseUrl {` and friends keep their bodies.
        if text[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            out.push_str(&text[pos..body_start]);
            pos = body_start;
            continue;
        }
        out.push_str(&text[pos..body_start]);
        // Field values are quoted and `"` is percent-encoded in every URL component, so a `}`
        // outside quotes closes the body. Braces are legal in a query or fragment, so the first
        // `}` in the text may sit inside one.
        let mut in_quotes = false;
        let end = text[body_start..]
            .char_indices()
            .find_map(|(i, c)| match c {
                '"' => {
                    in_quotes = !in_quotes;
                    None
                },
                '}' if !in_quotes => Some(i),
                _ => None,
            });
        let Some(k) = end else {
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

/// `scheme://host[:port]`, with `/***` appended when anything was removed.
pub fn redact_url(url: &Url) -> String {
    let scheme = url.scheme();
    let Some(host) = url.host() else {
        return format!("{scheme}:{REDACTED}");
    };
    let port = match url.port() {
        Some(port) => format!(":{port}"),
        None => String::new(),
    };
    let removed = !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.path(), "" | "/")
        || url.query().is_some()
        || url.fragment().is_some();
    let tail = match removed {
        true => format!("/{REDACTED}"),
        false => String::new(),
    };
    format!("{scheme}://{host}{port}{tail}")
}

pub fn redact_urls<'a>(urls: impl IntoIterator<Item = &'a Url>) -> Vec<String> {
    urls.into_iter().map(redact_url).collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_scrub() {
        let cases: &[(&str, &str)] = &[
            // The shape observed leaking in telemetry: reqwest's Debug.
            (
                r#"Transport(Custom(reqwest::Error { kind: Request, url: "https://rpc.example.com/v1/FAKEKEY", source: hyper::Error(IncompleteMessage) }))"#,
                r#"Transport(Custom(reqwest::Error { kind: Request, url: "https://rpc.example.com/***", source: hyper::Error(IncompleteMessage) }))"#,
            ),
            // reqwest's Display. The closing paren is absorbed, since `)` is legal in a path.
            (
                "error sending request for url (https://host/v1/FAKEKEY)",
                "error sending request for url (https://host/***",
            ),
            ("https://host/v2/FAKEKEY", "https://host/***"),
            ("https://host/rpc?apikey=FAKEKEY", "https://host/***"),
            ("https://host/p#frag", "https://host/***"),
            ("http://host:8545/k", "http://host:8545/***"),
            ("wss://host/v2/FAKEKEY", "wss://host/***"),
            ("https://[::1]:8545/FAKEKEY", "https://[::1]:8545/***"),
            ("https://user:pass@host/x", "https://host/***"),
            // Legal in a path or query, so these must not end the URL.
            ("https://host/a)b,FAKEKEY suffix", "https://host/*** suffix"),
            ("https://host/a|FAKEKEY", "https://host/***"),
            ("https://host/rpc?t={FAKEKEY}", "https://host/***"),
            (
                "a https://h1/k1 b https://h2/k2 c",
                "a https://h1/*** b https://h2/*** c",
            ),
            (
                "日本 https://host/FAKEKEY 中文",
                "日本 https://host/*** 中文",
            ),
            ("http://localhost:8545", "http://localhost:8545"),
            ("no url here", "no url here"),
            ("://", "://"),
            ("", ""),
        ];
        for (input, expected) in cases {
            assert_eq!(&scrub(input), expected, "input: {input:?}");
            let once = scrub(input);
            assert_eq!(scrub(&once), once, "not idempotent: {input:?}");
        }
    }

    /// The dominant shape in real telemetry: host and path are separate fields with no `://`, so
    /// URL-shaped matching misses it.
    #[test]
    fn test_scrub_url_debug() {
        let raw = format!(
            "urls: [{:?}]",
            Url::parse("https://rpc.example.com/v3/FAKEKEY").unwrap()
        );
        assert!(raw.contains("FAKEKEY"), "{raw}");
        assert!(!raw.contains("://"), "{raw}");

        let scrubbed = scrub(&raw);
        assert_eq!(scrubbed, "urls: [Url { *** }]");
        assert_eq!(scrub(&scrubbed), scrubbed);
    }

    /// Braces are legal in a query or fragment, so the first `}` may sit inside a field value.
    #[test]
    fn test_scrub_url_debug_brace_in_query_or_fragment() {
        for raw in [
            "https://rpc.example.com/v1?filter={a}&apikey=FAKEKEY",
            "https://rpc.example.com/v1#{a}FAKEKEY",
        ] {
            let dbg = format!("{:?}", Url::parse(raw).unwrap());
            assert_eq!(scrub_url_debug(&dbg), "Url { *** }", "input: {raw}");
        }
    }

    /// Only `url::Url` is collapsed; a type whose name merely ends in `Url` keeps its body.
    #[test]
    fn test_scrub_url_debug_requires_type_boundary() {
        let text = r#"BaseUrl { host: "keep.me" }"#;
        assert_eq!(scrub_url_debug(text), text);

        // Truncated debug output must not emit the partial body.
        let cut = r#"urls: [Url { scheme: "https", path: "/v3/FAKEKEY""#;
        assert!(!scrub_url_debug(cut).contains("FAKEKEY"));
    }

    #[test]
    fn test_redact_url() {
        let cases: &[(&str, &str)] = &[
            (
                "https://host.example.com/v1/FAKEKEY",
                "https://host.example.com/***",
            ),
            ("http://host:8545/k", "http://host:8545/***"),
            ("https://user:pass@host/x", "https://host/***"),
            ("https://host/rpc?apikey=FAKEKEY", "https://host/***"),
            ("https://[::1]:8545/FAKEKEY", "https://[::1]:8545/***"),
            ("http://localhost:8545", "http://localhost:8545"),
            ("mailto:foo@example.com", "mailto:***"),
        ];
        for (input, expected) in cases {
            let url = Url::parse(input).unwrap();
            assert_eq!(&redact_url(&url), expected, "input: {input:?}");
        }
    }
}
