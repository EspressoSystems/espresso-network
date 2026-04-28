use std::{env, fmt, sync::Once};

use hotshot::types::SignatureKey;
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

/// Install a tracing subscriber that routes through `print!` so `cargo test`
/// captures per-test output.  Logs only appear with `--nocapture` or for
/// failing tests via `--show-output`.  Honors `RUST_LOG` (defaulting to
/// `error` when unset).  Safe to call repeatedly.
pub fn init_test_logging() {
    LOG_INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error"));
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(use_color())
            .with_test_writer()
            .init();
    });
}

fn use_color() -> bool {
    env::var_os("NO_COLOR").is_none()
}

const KEY_PREFIX_LEN: usize = 19;

#[derive(Clone, Copy)]
pub struct KeyPrefix([u8; KEY_PREFIX_LEN]);

impl<K: SignatureKey> From<&K> for KeyPrefix {
    fn from(k: &K) -> Self {
        let s = k.to_string();
        let bytes = s.as_bytes();

        let mut buf = [0u8; KEY_PREFIX_LEN];
        let len = bytes.len().min(KEY_PREFIX_LEN);
        buf[..len].copy_from_slice(&bytes[..len]);

        Self(buf)
    }
}

impl fmt::Display for KeyPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = String::from_utf8_lossy(&self.0);
        f.write_str(&s)
    }
}
