use std::{
    borrow::Cow,
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

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
        match self {
            Self::Inet(ip, p) => self = Self::Inet(ip, p + o),
            Self::Name(hn, p) => self = Self::Name(hn, p + o),
        }
        self
    }

    pub fn is_ip(&self) -> bool {
        matches!(self, Self::Inet(..))
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
        let parse = |a: &str, p: Option<&str>| {
            let p: u16 = if let Some(p) = p {
                p.parse().map_err(|_| InvalidNetAddr(()))?
            } else {
                0
            };
            IpAddr::from_str(a)
                .map(|a| Self::Inet(a, p))
                .or_else(|_| Ok(Self::Name(a.to_string().into(), p)))
        };
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

impl Serialize for NetAddr {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(s)
    }
}

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

    use super::NetAddr;

    #[test]
    fn test_parse() {
        let a: NetAddr = "127.0.0.1:1234".parse().unwrap();
        let NetAddr::Inet(a, p) = a else {
            unreachable!()
        };
        assert_eq!(IpAddr::from([127, 0, 0, 1]), a);
        assert_eq!(1234, p);

        let a: NetAddr = "::1:1234".parse().unwrap();
        let NetAddr::Inet(a, p) = a else {
            unreachable!()
        };
        assert_eq!("::1".parse::<IpAddr>().unwrap(), a);
        assert_eq!(1234, p);

        let a: NetAddr = "localhost:1234".parse().unwrap();
        let NetAddr::Name(h, p) = a else {
            unreachable!()
        };
        assert_eq!("localhost", &h);
        assert_eq!(1234, p);

        let a: NetAddr = "sub.domain.com:1234".parse().unwrap();
        let NetAddr::Name(h, p) = a else {
            unreachable!()
        };
        assert_eq!("sub.domain.com", &h);
        assert_eq!(1234, p);
    }
}
