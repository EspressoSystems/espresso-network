//! Redaction of credentials embedded in provider URLs. L1 RPC providers put the API key in the
//! path or query (`https://host/v2/KEY`), so the host is kept and everything after it dropped. A
//! credential in the hostname is not redactable.
//!
//! [`scrub`] works on already-rendered text, which is the only handle available when the URL is
//! baked into someone else's `Debug`/`Display` (`reqwest` prints the full request URL in both).
//! [`redact_url`] takes a [`Url`] directly. Neither can reach `Url`'s own field-wise `Debug`
//! (`path: "/v2/KEY"`, no `://` anywhere); structs holding a [`Url`] redact that field themselves.
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
pub fn scrub(text: &str) -> String {
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
