use std::{env, fmt, sync::Once};

use tracing_subscriber::EnvFilter;

static LOG_INIT: Once = Once::new();

pub fn init_logging() {
    LOG_INIT.call_once(|| {
        if env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
            tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::from_default_env())
                .json()
                .init();
        } else {
            tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::from_default_env())
                .with_ansi(use_color())
                .init();
        }
    });
}

fn use_color() -> bool {
    env::var("NO_COLOR").map(|v| v.is_empty()).unwrap_or(true)
}

const KEY_PREFIX_LEN: usize = 19;

#[derive(Clone, Copy)]
pub struct KeyPrefix([u8; KEY_PREFIX_LEN]);

impl<K: fmt::Display + ?Sized> From<&K> for KeyPrefix {
    fn from(k: &K) -> Self {
        let s = k.to_string();
        let mut buf = [0u8; KEY_PREFIX_LEN];
        buf.copy_from_slice(&s.as_bytes()[..KEY_PREFIX_LEN]);
        Self(buf)
    }
}

impl fmt::Display for KeyPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(std::str::from_utf8(&self.0).unwrap())
    }
}
