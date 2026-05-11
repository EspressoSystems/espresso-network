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
}

impl fmt::Display for NetAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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

impl std::str::FromStr for NetAddr {
    type Err = InvalidNetAddr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(InvalidNetAddr(()));
        }

        let parse = |a: &str, p: Option<&str>| {
            let p: u16 = if let Some(p) = p {
                p.parse().map_err(|_| InvalidNetAddr(()))?
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
#[error("invalid network address")]
pub struct InvalidNetAddr(());

// TODO: distinguish human-readable:

#[cfg(feature = "serde")]
impl Serialize for NetAddr {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(s)
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
    use std::{iter::repeat_with, net::IpAddr};

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
}
