//! Minimal path resolution for the runuz hive — a self-contained
//! stand-in for hum's `hum-paths`. Follows the XDG Base Directory
//! spec, same defaults hum uses, so the hive shares humd's socket
//! and bee-key layout when run on the same machine.

use std::path::PathBuf;

/// Call once at startup to set unset XDG vars to HOME-relative defaults.
pub fn init() {
    let home = home();
    xdg_default("XDG_STATE_HOME",  home.join(".local/state"));
    xdg_default("XDG_CONFIG_HOME", home.join(".config"));
    xdg_default("XDG_DATA_HOME",   home.join(".local/share"));
    xdg_default("XDG_CACHE_HOME",  home.join(".cache"));
    xdg_default("XDG_RUNTIME_DIR", home.join(".local/state/run"));
}

fn xdg_default(var: &str, default: PathBuf) {
    if std::env::var_os(var).is_none() {
        unsafe { std::env::set_var(var, default); }
    }
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).expect("HOME must be set")
}

fn xdg(var: &str) -> PathBuf {
    if let Some(v) = std::env::var_os(var) {
        return PathBuf::from(v);
    }
    init();
    PathBuf::from(std::env::var_os(var).expect("init() set the var"))
}

/// `$XDG_STATE_HOME/hum` — persistent state (bee keys, the thrum socket).
pub fn state_dir() -> PathBuf { xdg("XDG_STATE_HOME").join("hum") }

/// Directory holding per-bee ed25519 identity seeds.
pub fn bees_dir() -> PathBuf { state_dir().join("bees") }

/// Per-bee ed25519 identity seed; one file per hive kind.
pub fn bee_key(kind: &str) -> PathBuf { bees_dir().join(format!("{kind}.key")) }

pub const THRUM_SOCK_BASENAME: &str = "thrum.sock";
pub const RUNTIME_INFO_BASENAME: &str = "runtime.json";

/// Default thrum socket path (what humd BINDS unless overridden).
pub fn thrum_sock() -> PathBuf {
    if let Some(p) = std::env::var_os("HUM_THRUM_SOCK") { return PathBuf::from(p); }
    if let Some(p) = std::env::var_os("HUM_SOCKET")     { return PathBuf::from(p); }
    state_dir().join(THRUM_SOCK_BASENAME)
}

/// What clients should connect to: honors humd's rendezvous
/// `runtime.json` first, then env overrides, then the default.
pub fn thrum_sock_resolved() -> PathBuf {
    if let Some(p) = std::env::var_os("HUM_THRUM_SOCK") { return PathBuf::from(p); }
    if let Some(p) = std::env::var_os("HUM_SOCKET")     { return PathBuf::from(p); }
    if let Some(rt) = read_runtime_info() { return rt.socket; }
    state_dir().join(THRUM_SOCK_BASENAME)
}

pub fn runtime_info() -> PathBuf { state_dir().join(RUNTIME_INFO_BASENAME) }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RuntimeInfo {
    pub socket: PathBuf,
    pub pid: u32,
    pub version: String,
    pub thrum_version: String,
}

pub fn read_runtime_info() -> Option<RuntimeInfo> {
    let raw = std::fs::read_to_string(runtime_info()).ok()?;
    serde_json::from_str(&raw).ok()
}
