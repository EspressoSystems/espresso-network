use std::{
    borrow::Cow,
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// A network address.
///
/// Either an IP address and port number or else a hostname and port number.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NetAddr {
    Inet(IpAddr, u16),
    Name(Cow<'static, str>, u16),
}

impl NetAddr {
    pub fn named<S>(name: S, port: u16) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        Self::Name(name.into(), port)
    }

    /// Get the port number of an address.
    pub fn port(&self) -> u16 {
        match self {
            Self::Inet(_, p) => *p,
            Self::Name(_, p) => *p,
        }
    }

    /// Set the address port.
    pub fn set_port(&mut self, p: u16) {
        match self {
            Self::Inet(_, o) => *o = p,
            Self::Name(_, o) => *o = p,
        }
    }

    pub fn with_port(mut self, p: u16) -> Self {
        match self {
            Self::Inet(ip, _) => self = Self::Inet(ip, p),
            Self::Name(hn, _) => self = Self::Name(hn, p),
        }
        self
    }

    pub fn with_offset(mut self, o: u16) -> Self {
        debug_assert!(self.port().checked_add(o).is_some());
        match self {
            Self::Inet(ip, p) => self = Self::Inet(ip, p + o),
            Self::Name(hn, p) => self = Self::Name(hn, p + o),
        }
        self
    }

    pub fn is_ip(&self) -> bool {
        matches!(self, Self::Inet(..))
    }

    /// Whether this address is plausibly publicly routable. Returns `false` for IP literals
    /// in non-globally-routable ranges (loopback, unspecified, RFC 1918 private, link-local,
    /// broadcast, documentation, IPv6 multicast) and the literal `localhost`. Other hostnames
    /// are trusted and return `true`. Approximates the (still unstable) `IpAddr::is_global`
    /// using stable predicates; the IPv6 surface is incomplete (`fe80::/10` link-local and
    /// `fc00::/7` unique-local addresses are treated as global here).
    pub fn is_probably_global(&self) -> bool {
        match self {
            Self::Inet(IpAddr::V4(v4), _) => {
                !(v4.is_loopback()
                    || v4.is_unspecified()
                    || v4.is_private()
                    || v4.is_link_local()
                    || v4.is_broadcast()
                    || v4.is_documentation())
            },
            Self::Inet(IpAddr::V6(v6), _) => {
                !(v6.is_loopback() || v6.is_unspecified() || v6.is_multicast())
            },
            Self::Name(host, _) => !host.eq_ignore_ascii_case("localhost"),
        }
    }

    /// Checks that this address is well-formed.
    ///
    /// A hostname is dot-separated labels, each 1 to 63 characters of ASCII letters,
    /// digits, `-` and `_`, not beginning or ending with `-`, and not digits and dots
    /// alone. RFC 952 and RFC 1123 do not allow `_`, but names using it are in use. An
    /// IP address must be the address it prints as, so an IPv4 address in IPv6 form is
    /// not well-formed; [`IpAddr::to_canonical`] converts it.
    ///
    /// Neither the port nor the length of the whole name is checked.
    pub fn validate(&self) -> Result<(), InvalidNetAddr> {
        match self {
            Self::Inet(ip, _) => {
                if ip.to_canonical() != *ip {
                    return Err(InvalidNetAddr("IPv4 address in IPv6 form"));
                }
                Ok(())
            },
            Self::Name(host, _) => {
                const MAX_LABEL_LEN: usize = 63;

                if host.is_empty() {
                    return Err(InvalidNetAddr("empty hostname"));
                }
                if host.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
                    return Err(InvalidNetAddr("host is not an IP address nor a hostname"));
                }
                for label in host.split('.') {
                    if label.is_empty() {
                        return Err(InvalidNetAddr("hostname contains invalid dots"));
                    }
                    if label.len() > MAX_LABEL_LEN {
                        return Err(InvalidNetAddr("hostname part is longer than 63 chars"));
                    }
                    if label.starts_with('-') || label.ends_with('-') {
                        return Err(InvalidNetAddr("hostname part starts or ends with `-`"));
                    }
                    if !label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                    {
                        return Err(InvalidNetAddr("hostname contains invalid characters"));
                    }
                }
                Ok(())
            },
        }
    }

    /// The address without brackets around an IPv6 literal.
    ///
    /// This is how [`fmt::Display`] printed every address before IPv6 literals were
    /// bracketed. The format is retained here for backwards compatibility.
    pub fn unbracketed_string(&self) -> String {
        match self {
            Self::Inet(a, p) => format!("{a}:{p}"),
            Self::Name(h, p) => format!("{h}:{p}"),
        }
    }
}

impl fmt::Display for NetAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inet(a @ IpAddr::V6(_), p) => write!(f, "[{a}]:{p}"),
            Self::Inet(a, p) => write!(f, "{a}:{p}"),
            Self::Name(h, p) => write!(f, "{h}:{p}"),
        }
    }
}

impl From<(&str, u16)> for NetAddr {
    fn from((h, p): (&str, u16)) -> Self {
        Self::Name(h.to_string().into(), p)
    }
}

impl From<(String, u16)> for NetAddr {
    fn from((h, p): (String, u16)) -> Self {
        Self::Name(h.into(), p)
    }
}

impl From<(IpAddr, u16)> for NetAddr {
    fn from((ip, p): (IpAddr, u16)) -> Self {
        Self::Inet(ip, p)
    }
}

impl From<(Ipv4Addr, u16)> for NetAddr {
    fn from((ip, p): (Ipv4Addr, u16)) -> Self {
        Self::Inet(IpAddr::V4(ip), p)
    }
}

impl From<(Ipv6Addr, u16)> for NetAddr {
    fn from((ip, p): (Ipv6Addr, u16)) -> Self {
        Self::Inet(IpAddr::V6(ip), p)
    }
}

impl From<SocketAddr> for NetAddr {
    fn from(a: SocketAddr) -> Self {
        Self::Inet(a.ip(), a.port())
    }
}

/// Grammar:
///
/// ```text
/// addr = host               -- port is 0
///      | host ":" port
///      | "[" host "]"       -- port is 0
///      | "[" host "]:" port
///
/// host = IP | name
/// IP   = <std::net::IpAddr>
/// name = <any sequence of characters, possibly empty>
/// port = <u16>
/// ```
///
/// - Input starting with `[` has the port after the last `]:`. With no `]:` anywhere there
///   is no port, so `[a:80` is the name `[a:80` on port 0.
/// - Input not starting with `[` has the port after the last `:` or defaults to 0 if there
///   is no `:`.
/// - `host` is an IP if `IpAddr` parses it, a name otherwise.
/// - Parsing the port permits a leading `+` and leading zeros.
impl std::str::FromStr for NetAddr {
    type Err = InvalidNetAddr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(InvalidNetAddr("the address is empty"));
        }

        let parse = |a: &str, p: Option<&str>| {
            let p: u16 = if let Some(p) = p {
                p.parse().map_err(|_| InvalidNetAddr("invalid port"))?
            } else {
                0
            };
            // Strip brackets from IPv6 addresses like `[::1]`.
            let a = if a.starts_with('[') && a.ends_with(']') {
                &a[1..a.len() - 1]
            } else {
                a
            };
            IpAddr::from_str(a)
                .map(|a| Self::Inet(a, p))
                .or_else(|_| Ok(Self::Name(a.to_string().into(), p)))
        };

        // Handle bracketed IPv6 like `[::1]:8080` or `[::1]` (no port).
        if s.starts_with('[') {
            return match s.rfind("]:") {
                Some(i) => parse(&s[..i + 1], Some(&s[i + 2..])),
                None => parse(s, None),
            };
        }

        match s.rsplit_once(':') {
            None => parse(s, None),
            Some((a, p)) => parse(a, Some(p)),
        }
    }
}

impl TryFrom<&str> for NetAddr {
    type Error = InvalidNetAddr;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        val.parse()
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid network address: {0}")]
pub struct InvalidNetAddr(&'static str);

// TODO: distinguish human-readable:

#[cfg(feature = "serde")]
impl Serialize for NetAddr {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.unbracketed_string().serialize(s)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for NetAddr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        let a = s.parse().map_err(de::Error::custom)?;
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        iter::repeat_with,
        net::{IpAddr, SocketAddr},
    };

    use quickcheck::{Arbitrary, Gen, quickcheck};

    use super::NetAddr;

    impl Arbitrary for NetAddr {
        fn arbitrary(g: &mut Gen) -> Self {
            let port = u16::arbitrary(g);
            if bool::arbitrary(g) {
                let len = u8::arbitrary(g);
                let host: String = repeat_with(|| char::arbitrary(g))
                    .filter(|c| !"[]".contains(*c))
                    .take(len.into())
                    .collect();
                NetAddr::Name(host.into(), port)
            } else {
                let ip = IpAddr::arbitrary(g);
                NetAddr::Inet(ip, port)
            }
        }
    }

    quickcheck! {
        fn prop_to_string_parse_identity(a: NetAddr) -> bool {
            a.to_string().parse().ok() == Some(a)
        }
    }

    #[test]
    fn empty_is_invalid() {
        assert!("".parse::<NetAddr>().is_err())
    }

    #[test]
    fn validate_accepts() {
        let cases: &[&str] = &[
            // an IP address, however it was written
            "1.2.3.4:8080",
            "1.2.3.4:0",
            "1.2.3.4:65535",
            "[::1]:9977",
            "::1:9977",
            "[2001:db8::1]:9000",
            "2001:db8::1:9000",
            "[::]:80",
            ":::80",
            "[::ffff:0:102:304]:80",
            "[1.2.3.4]:80",
            // hostnames, in any case, and with or without a port
            "localhost:1234",
            "example.com",
            "a-b.c:80",
            "a_b.example.com:80",
            "_svc.example.com:80",
            "Node.Example.COM:8080",
            "[node.example.com]:8080",
            "crpk232f2b1uqepj3qmg.bdnodes.net:9977",
        ];
        for s in cases {
            let a: NetAddr = s.parse().unwrap_or_else(|_| panic!("parse {s}"));
            assert_eq!(a.validate().map_err(|e| e.to_string()), Ok(()), "for {s}");
        }
    }

    #[test]
    fn validate_rejects() {
        let cases: &[&str] = &[
            // an IPv6 address without a port is a hostname with an empty part
            "2001:db8::1",
            "::1",
            "[foo:bar]:80",
            "[::1]:80]:90",
            "[]:7",
            // digits and dots that are not an IP address
            "01.2.3.4:80",
            "1.2.3.4.5:80",
            "127.1:1",
            "12345:80",
            // hostname parts
            "example.com.:80",
            ".example.com:80",
            "a..b:80",
            "-a.b:80",
            "a-.b:80",
            "a b:80",
            "ä:80",
            "%:1",
            "a/b:80",
        ];
        for s in cases {
            let a: NetAddr = s.parse().unwrap_or_else(|_| panic!("parse {s}"));
            assert!(a.validate().is_err(), "should be rejected: {s:?}");
        }

        // An IPv4 address in IPv6 form is one address written two ways.
        let mapped: NetAddr = "[::ffff:1.2.3.4]:80".parse().expect("parse");
        assert!(mapped.validate().is_err());
        assert!(
            mapped
                .validate()
                .unwrap_err()
                .to_string()
                .contains("IPv4 address in IPv6 form")
        );
        let NetAddr::Inet(ip, port) = mapped else {
            panic!("an IP address")
        };
        assert_eq!(
            NetAddr::Inet(ip.to_canonical(), port),
            "1.2.3.4:80".parse().expect("parse")
        );
    }

    #[test]
    fn test_is_probably_global() {
        let cases: &[(&str, bool)] = &[
            ("127.0.0.1:1234", false),
            ("0.0.0.0:1234", false),
            ("10.0.0.1:1234", false),
            ("172.16.5.4:1234", false),
            ("192.168.1.1:1234", false),
            ("169.254.0.1:1234", false),
            ("255.255.255.255:1234", false),
            ("192.0.2.1:1234", false),
            ("::1:1234", false),
            (":::1234", false),
            ("ff00::1:1234", false),
            ("localhost:1234", false),
            ("LOCALHOST:1234", false),
            ("8.8.8.8:1234", true),
            ("1.1.1.1:1234", true),
            ("2606:4700:4700::1111:1234", true),
            ("example.com:1234", true),
            ("node.internal:1234", true),
        ];
        for (s, expected) in cases {
            let a: NetAddr = s.parse().unwrap_or_else(|_| panic!("parse {s}"));
            assert_eq!(a.is_probably_global(), *expected, "for input {s}");
        }
    }

    #[test]
    fn ipv6_prints_as_a_socket_addr() {
        let a: NetAddr = "::1:9977".parse().expect("parse");
        assert_eq!(a.to_string(), "[::1]:9977");
        assert!(a.to_string().parse::<SocketAddr>().is_ok());
        assert!("::1:9977".parse::<SocketAddr>().is_err());
    }
}
