use std::{
    borrow::Cow,
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    ops::Deref,
    str::FromStr,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Dot-separated ASCII labels.
///
/// A label is 1 to 63 characters of ASCII letters, digits, `-` and `_`, and may
/// not begin or end with `-`. The whole name is at most 253 characters and is
/// not made of digits and dots alone.
///
/// RFC 952 and RFC 1123 do not allow `_` but it is allowed here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hostname(Cow<'static, str>);

impl Hostname {
    /// The hostname `name`, or `None` if that is not a hostname.
    pub fn new<S>(name: S) -> Option<Self>
    where
        S: Into<Cow<'static, str>>,
    {
        let name = name.into();
        is_hostname(&name).then_some(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for Hostname {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for Hostname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_hostname(s: &str) -> bool {
    if s.is_empty() || s.len() > 253 {
        return false;
    }
    // Digits and dots alone is an address, not a name.
    if s.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return false;
    }
    s.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    })
}

fn parse_port(s: &str) -> Result<u16, InvalidNetAddr> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(InvalidNetAddr(()));
    }
    if s.len() > 1 && s.starts_with('0') {
        return Err(InvalidNetAddr(()));
    }
    s.parse().map_err(|_| InvalidNetAddr(()))
}

/// An IP literal written in one, canonical way.
fn canonical_ip(text: &str) -> Option<IpAddr> {
    let ip = IpAddr::from_str(text).ok()?;
    // Ipv4Addr's Display impl conforms to RFC 5952.
    (ip.to_string() == text && ip.to_canonical() == ip).then_some(ip)
}

/// A network address.
///
/// Either an IP address and port number or else a hostname and port number.
///
/// # Syntax
///
/// `host:port` where host = IPv4 literal | "[" IPv6 literal "]" | hostname
///
/// The port is decimal with no sign and no leading zeros. An IP literal is written the
/// one way ([`canonical_ip`]). Brackets are for IPv6 and required, so bare `::1:8080` is
/// rejected along with `[1.2.3.4]:5`, `[node.example.com]:8080` and an unclosed bracket.
/// A hostname is a [`Hostname`].
///
/// # Printing
///
/// [`fmt::Display`] brackets an IPv6 literal, so printing and parsing are inverse:
/// every address prints to a string that parses back to it, and every string this
/// accepts prints back to itself.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NetAddr {
    Inet(IpAddr, u16),
    Name(Hostname, u16),
}

impl NetAddr {
    /// The address `host:port`, where `host` is not bracketed.
    ///
    /// An IP literal gives [`NetAddr::Inet`] and a hostname gives [`NetAddr::Name`];
    /// anything else is an error.
    pub fn host_port(host: &str, port: u16) -> Result<Self, InvalidNetAddr> {
        if IpAddr::from_str(host).is_ok() {
            return canonical_ip(host)
                .map(|ip| Self::Inet(ip, port))
                .ok_or(InvalidNetAddr(()));
        }
        Hostname::new(host.to_string())
            .map(|h| Self::Name(h, port))
            .ok_or(InvalidNetAddr(()))
    }

    /// The address `name:port`, where `name` is a hostname.
    ///
    /// # Panics
    ///
    /// When `name` is not a hostname, which includes every IP literal.
    pub fn named<S>(name: S, port: u16) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        Hostname::new(name)
            .map(|h| Self::Name(h, port))
            .expect("valid hostname")
    }

    pub fn port(&self) -> u16 {
        match self {
            Self::Inet(_, p) => *p,
            Self::Name(_, p) => *p,
        }
    }

    pub fn set_port(&mut self, p: u16) {
        match self {
            Self::Inet(_, o) => *o = p,
            Self::Name(_, o) => *o = p,
        }
    }

    pub fn with_port(mut self, p: u16) -> Self {
        self.set_port(p);
        self
    }

    pub fn with_offset(mut self, o: u16) -> Self {
        debug_assert!(self.port().checked_add(o).is_some());
        let p = self.port() + o;
        self.set_port(p);
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
            Self::Name(host, _) => !host.as_str().eq_ignore_ascii_case("localhost"),
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

impl From<(IpAddr, u16)> for NetAddr {
    fn from((ip, p): (IpAddr, u16)) -> Self {
        Self::Inet(ip.to_canonical(), p)
    }
}

impl From<(Ipv4Addr, u16)> for NetAddr {
    fn from((ip, p): (Ipv4Addr, u16)) -> Self {
        Self::Inet(IpAddr::V4(ip), p)
    }
}

impl From<(Ipv6Addr, u16)> for NetAddr {
    fn from((ip, p): (Ipv6Addr, u16)) -> Self {
        Self::from((IpAddr::V6(ip), p))
    }
}

impl From<SocketAddr> for NetAddr {
    fn from(a: SocketAddr) -> Self {
        Self::from((a.ip(), a.port()))
    }
}

impl TryFrom<(&str, u16)> for NetAddr {
    type Error = InvalidNetAddr;

    fn try_from((h, p): (&str, u16)) -> Result<Self, Self::Error> {
        Self::host_port(h, p)
    }
}

impl FromStr for NetAddr {
    type Err = InvalidNetAddr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix('[') {
            let i = rest.rfind("]:").ok_or(InvalidNetAddr(()))?;
            let port = parse_port(&rest[i + 2..])?;
            if let Some(ip @ IpAddr::V6(_)) = canonical_ip(&rest[..i]) {
                Ok(Self::Inet(ip, port))
            } else {
                Err(InvalidNetAddr(()))
            }
        } else {
            let (host, port) = s.rsplit_once(':').ok_or(InvalidNetAddr(()))?;
            let port = parse_port(port)?;
            if matches!(IpAddr::from_str(host), Ok(IpAddr::V6(_))) {
                return Err(InvalidNetAddr(()));
            }
            Self::host_port(host, port)
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
    use std::net::IpAddr;

    use quickcheck::{Arbitrary, Gen, quickcheck};

    use super::{Hostname, NetAddr, is_hostname};

    impl Arbitrary for NetAddr {
        fn arbitrary(g: &mut Gen) -> Self {
            let port = u16::arbitrary(g);
            if bool::arbitrary(g) {
                NetAddr::Name(arbitrary_hostname(g), port)
            } else {
                NetAddr::from((IpAddr::arbitrary(g), port))
            }
        }
    }

    /// A label: allowed characters, never empty, never hyphen-bounded.
    fn arbitrary_label(g: &mut Gen) -> String {
        const MID: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789-_";
        const EDGE: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789_";
        let mut l = String::new();
        l.push(char::from(*g.choose(EDGE).expect("non-empty")));
        for _ in 0..(u8::arbitrary(g) % 8) {
            l.push(char::from(*g.choose(MID).expect("non-empty")));
        }
        l.push(char::from(*g.choose(EDGE).expect("non-empty")));
        l
    }

    /// A hostname built to the rules, so that the properties below are about addresses
    /// that exist rather than about strings that could never be one.
    fn arbitrary_hostname(g: &mut Gen) -> Hostname {
        let n = u8::arbitrary(g) % 3 + 1;
        let mut labels: Vec<String> = (0..n).map(|_| arbitrary_label(g)).collect();
        labels[0].insert(0, 'h');
        let name = labels.join(".");
        Hostname::new(name.clone()).unwrap_or_else(|| panic!("built a bad hostname: {name}"))
    }

    quickcheck! {
        /// Printing and parsing are inverse, with no side condition: every address
        /// that can be built prints to a string that parses back to it.
        fn prop_to_string_parse_identity(a: NetAddr) -> bool {
            a.to_string().parse().ok() == Some(a)
        }

        /// And parsing then printing gives the string back, so an address has one
        /// spelling.
        fn prop_parse_to_string_identity(a: NetAddr) -> bool {
            let s = a.to_string();
            s.parse::<NetAddr>().ok().map(|b| b.to_string()) == Some(s)
        }

        /// No hostname is an IP literal, so a `Name` never prints as an address.
        fn prop_hostname_is_not_a_literal(a: NetAddr) -> bool {
            match a {
                NetAddr::Name(h, _) => h.as_str().parse::<IpAddr>().is_err(),
                NetAddr::Inet(..) => true,
            }
        }
    }

    #[test]
    fn accepted() {
        let cases: &[(&str, &str)] = &[
            // IPv4
            ("1.2.3.4:8080", "1.2.3.4:8080"),
            ("1.2.3.4:0", "1.2.3.4:0"),
            ("1.2.3.4:65535", "1.2.3.4:65535"),
            ("127.0.0.1:1", "127.0.0.1:1"),
            // IPv6, always bracketed
            ("[::1]:8080", "[::1]:8080"),
            ("[2001:db8::1]:9000", "[2001:db8::1]:9000"),
            ("[::]:80", "[::]:80"),
            ("[2001:db8::]:5", "[2001:db8::]:5"),
            // hostnames
            ("localhost:1234", "localhost:1234"),
            ("node.example.com:8080", "node.example.com:8080"),
            ("a_b.example.com:80", "a_b.example.com:80"),
            ("_svc.example.com:80", "_svc.example.com:80"),
            ("a-b.c:80", "a-b.c:80"),
            (
                "crpk232f2b1uqepj3qmg.bdnodes.net:9977",
                "crpk232f2b1uqepj3qmg.bdnodes.net:9977",
            ),
        ];
        for (input, printed) in cases {
            let a: NetAddr = input.parse().unwrap_or_else(|_| panic!("parse {input}"));
            assert_eq!(a.to_string(), *printed, "printing {input}");
            assert_eq!(
                a.to_string().parse::<NetAddr>().ok(),
                Some(a),
                "round trip {input}"
            );
        }
    }

    #[test]
    fn rejected() {
        let cases: &[&str] = &[
            // nothing, and no port
            "",
            ":",
            "a",
            "a:",
            "1.2.3.4",
            "[::1]",
            "[a]",
            // an unclosed bracket
            "[2001:db8::1:9000",
            // an IPv6 literal not in brackets: `[::1]:8080` is the only spelling
            "::1:8080",
            "2001:db8::1:9000",
            ":::80",
            "2001:db8:::5",
            "::ffff:1.2.3.4:80",
            "[weird",
            "[::1",
            "[a]b:1",
            // brackets are only for IPv6
            "[1.2.3.4]:5",
            "[node.example.com]:8080",
            "[x]:5",
            "[a]:5",
            "[]:7",
            "[[a]]:5",
            "[::1]:80]:90",
            // an IPv6 literal not written the way it prints
            "[2001:0DB8::0001]:80",
            "[::0001]:80",
            // an IPv4-mapped IPv6 address: write the IPv4 one
            "::ffff:1.2.3.4:80",
            "[::ffff:1.2.3.4]:80",
            // a host that is neither a literal nor a hostname
            "x]:5",
            "a]:5",
            "a[:5",
            "a:b:1",
            "1:2:3",
            "%:1",
            "ä:1",
            "a b:80",
            "2606:4700:4700::1111",
            "::1",
            "::ffff:1.2.3.4",
            "2001:db8::",
            // a mistyped address is not a name
            "01.2.3.4:80",
            "127.1:1",
            "1.2.3.4.5:80",
            "12345:80",
            // hostname labels
            "example.com.:80",
            "a..b:80",
            "-a.b:80",
            "a-.b:80",
            ".a:80",
            // the port
            "a:65536",
            "a:+80",
            "a:080",
            "a:00",
            "a:+0",
            "a:-1",
            "a: 80",
            "a:8_0",
            "host:notaport",
            "[::1]:bad",
            "[::1]:",
            "[a]:1:2",
        ];
        for s in cases {
            assert!(s.parse::<NetAddr>().is_err(), "should be rejected: {s:?}");
        }
    }

    #[test]
    fn no_hostname_is_an_ip_literal() {
        for s in [
            "1.2.3.4",
            "::1",
            "::",
            "2001:db8::1",
            "255.255.255.255",
            "0.0.0.0",
        ] {
            assert!(!is_hostname(s), "{s:?} must not be a hostname");
        }
    }

    /// One host, one address, one string: the mapped spelling is refused, and an
    /// address that arrives in that form is folded rather than kept.
    #[test]
    fn ipv4_mapped_is_the_ipv4_address() {
        assert!("::ffff:1.2.3.4:80".parse::<NetAddr>().is_err());
        assert!("[::ffff:1.2.3.4]:80".parse::<NetAddr>().is_err());
        let v4: NetAddr = "1.2.3.4:80".parse().unwrap();
        // However it arrives, it is the same value, and it prints as the IPv4 address.
        let mapped: std::net::SocketAddr = "[::ffff:1.2.3.4]:80".parse().unwrap();
        assert_eq!(NetAddr::from(mapped), v4);
        assert_eq!(NetAddr::from(mapped).to_string(), "1.2.3.4:80");
        let ip: IpAddr = "::ffff:1.2.3.4".parse().unwrap();
        assert_eq!(NetAddr::from((ip, 80)), v4);
        // An IPv4-translated address is not mapped, and stays IPv6.
        let translated: NetAddr = "[::ffff:0:102:304]:80".parse().unwrap();
        assert!(!translated.is_ip() || translated.to_string() == "[::ffff:0:102:304]:80");
    }

    #[test]
    fn ipv6_prints_the_way_everything_else_reads_it() {
        let a: NetAddr = "[::1]:8080".parse().unwrap();
        assert_eq!(a.to_string(), "[::1]:8080");
        assert!(a.to_string().parse::<std::net::SocketAddr>().is_ok());
        // The form printed before, which `SocketAddr` rejects and so do we: an address
        // has one spelling in both directions.
        assert!("::1:8080".parse::<std::net::SocketAddr>().is_err());
        assert!("::1:8080".parse::<NetAddr>().is_err());
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
            ("[::1]:1234", false),
            ("[::]:1234", false),
            ("[ff00::1]:1234", false),
            ("localhost:1234", false),
            ("LOCALHOST:1234", false),
            ("8.8.8.8:1234", true),
            ("1.1.1.1:1234", true),
            ("[2606:4700:4700::1111]:1234", true),
            ("example.com:1234", true),
            ("node.internal:1234", true),
        ];
        for (s, expected) in cases {
            let a: NetAddr = s.parse().unwrap_or_else(|_| panic!("parse {s}"));
            assert_eq!(a.is_probably_global(), *expected, "for input {s}");
        }
    }
}
