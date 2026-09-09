use http::{HeaderMap, header};

/// Content types supported by the wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentType {
    Json,
    Binary,
}

impl ContentType {
    /// The MIME type this format is named by in `Content-Type` and `Accept` headers.
    pub fn mime(self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Binary => "application/octet-stream",
        }
    }

    /// Parse a `Content-Type` header value.
    ///
    /// Only the media type's essence is compared: parameters (e.g. a charset appended by a proxy)
    /// and case are tolerated per RFC 9110.
    ///
    /// Note the asymmetry with [`wants_binary`]: `Content-Type` names one format and is parsed
    /// strictly by essence, while `Accept` can be a list, so negotiation is a loose substring
    /// match.
    pub fn parse(header_value: &str) -> Option<Self> {
        let essence = header_value
            .split_once(';')
            .map_or(header_value, |(essence, _)| essence)
            .trim()
            .to_ascii_lowercase();
        match essence.as_str() {
            "application/json" => Some(Self::Json),
            "application/octet-stream" => Some(Self::Binary),
            _ => None,
        }
    }

    /// The response format negotiated by a request's `Accept` header: binary iff the request
    /// [`wants_binary`], JSON otherwise.
    pub fn negotiate(headers: &HeaderMap) -> Self {
        if wants_binary(headers) {
            Self::Binary
        } else {
            Self::Json
        }
    }
}

/// Whether a request negotiates VBS binary responses. Internal clients default to
/// `Accept: application/octet-stream`; everything else (browsers, curl) gets JSON.
///
/// A loose substring match, since `Accept` can be a list (see the asymmetry note on
/// [`ContentType::parse`]).
pub fn wants_binary(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/octet-stream"))
}
