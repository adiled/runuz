use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::ToolResult;
use serde_json::json;

pub(crate) fn splice(source: &str, start: usize, end: usize, with: &str) -> String {
    let mut out = String::with_capacity(source.len() + with.len());
    out.push_str(&source[..start]);
    out.push_str(with);
    out.push_str(&source[end..]);
    out
}

pub(crate) fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let f = std::fs::OpenOptions::new().read(true).write(true).create(true).open(path)?;
    let fd = f.as_raw_fd();
    unsafe {
        if libc::flock(fd, libc::LOCK_EX) != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    unsafe {
        libc::flock(fd, libc::LOCK_UN);
    }
    Ok(())
}

pub(crate) fn ok(msg: String, path: &Path) -> ToolResult {
    ToolResult {
        output: msg,
        title: Some(path.display().to_string()),
        metadata: Some(json!({ "path": path.display().to_string() })),
        is_error: false,
    }
}

#[cfg(test)]
pub(crate) fn tmp(seq: &AtomicUsize, tag: &str, ext: &str) -> PathBuf {
    let n = seq.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("runuz-{tag}-{}-{}.{}", std::process::id(), n, ext))
}