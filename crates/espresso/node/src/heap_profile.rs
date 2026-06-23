use std::{ffi::CString, path::Path, process, time::Duration};

use anyhow::{Context, Result};
use tikv_jemalloc_ctl::raw;
use tokio::time::interval;
use tracing::{error, info};

pub async fn dump_every(freq: Duration, dir: &Path) {
    let pid = process::id();
    let mut ticker = interval(freq);
    ticker.tick().await; // ignore first tick which is ready immediately

    for i in 0.. {
        ticker.tick().await;
        let path = dir.join(format!("espresso-heap.{pid}.{i}.heap"));
        match write(&path) {
            Ok(()) => info!(target: "announce", ?path, "wrote jemalloc heap dump"),
            Err(err) => error!(?path, %err, "failed to write jemalloc heap dump"),
        }
    }
}

fn write(path: &Path) -> Result<()> {
    let cpath = CString::new(path.to_str().context("invalid path")?)?;
    unsafe { raw::write(b"prof.dump\0", cpath.as_ptr())? };
    Ok(())
}
