//! Self-update: the latest Kraken Mount release on GitHub, checked once per start and, on request
//! from the web UI, downloaded and sent to elfldr. The new copy then takes over the web port
//! (web::take_port asks this copy to quit) and saves its own self copy.

use crate::{get_retry, is_module, newer, parse_release, Release};
use std::io::Write;
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

// Kraken Mount Plus updates from its own releases, so an update never swaps it for upstream Kraken Mount.
const RELEASE_URL: &str = "https://api.github.com/repos/sakulsol/kraken-mount-plus/releases/latest";
const ASSET: &str = "kraken-mount-plus.elf";
const MAX: usize = 32 << 20;
const ELFLDR: ([u8; 4], u16) = ([127, 0, 0, 1], 9021);
const RUNNING: &str = env!("CARGO_PKG_VERSION");

/// The installable newer release found at start, None until (or unless) one is found.
static LATEST: Mutex<Option<Release>> = Mutex::new(None);
/// One install at a time.
static UPDATING: AtomicBool = AtomicBool::new(false);
/// sys::uptime_ms until which the scan loop starts no pass after a send (0 = none), so no build is
/// cut short when the new copy quits this one. It runs out on its own, so a new copy that never
/// takes over only delays the scan.
static HOLD_UNTIL: AtomicU64 = AtomicU64::new(0);
const HOLD_MS: u64 = 90_000;

fn sent_recently() -> bool {
    HOLD_UNTIL.load(Ordering::SeqCst) > crate::sys::uptime_ms()
}

/// An install is in flight or its ELF was sent less than HOLD_MS ago: no scan pass, no second install.
pub fn holding() -> bool {
    UPDATING.load(Ordering::SeqCst) || sent_recently()
}

/// `rel` when it is newer than `running` and its asset is not empty.
fn keep(rel: Release, running: &str) -> Option<Release> {
    (newer(Some(&rel.tag), Some(running)) && rel.size > 0).then_some(rel)
}

/// Background check of the latest release: up to 3 tries 20 s apart, once per start, always on.
pub fn spawn_check() {
    std::thread::spawn(|| {
        let r = get_retry("update", RELEASE_URL, 1 << 20, 3)
            .map_err(|e| format!("GET {RELEASE_URL}: {e}"))
            .and_then(|b| parse_release(&b, ASSET).map_err(|e| format!("{RELEASE_URL}: {e}")));
        match r {
            Ok(rel) => {
                log!("update: latest {} (running {RUNNING})", rel.tag);
                *LATEST.lock().unwrap() = keep(rel, RUNNING);
            }
            Err(e) => log!("update: {e}"),
        }
    });
}

/// `{version, size}` of the installable newer release, None when there is none.
pub fn available() -> Option<serde_json::Value> {
    LATEST.lock().unwrap().as_ref().map(|r| serde_json::json!({"version": r.tag, "size": r.size}))
}

/// Downloads release `version` (it must be the one found at start) and sends it to elfldr.
/// `paused`: whether a game runs (the new copy's take_port would be refused), asked before the
/// download and again before the send. Err is (HTTP status, text):
/// 400 not the found release, 409 a game or another update runs, 503 elfldr does not answer,
/// 502 the download or the send failed.
/// `pass_idle`: no scan pass holds State::work (asked once the hold is set: a later pass then waits).
pub fn install(version: &str, paused: &dyn Fn() -> bool, pass_idle: &dyn Fn() -> bool) -> Result<(), (u16, String)> {
    let rel = LATEST.lock().unwrap().as_ref().map(|r| Release { tag: r.tag.clone(), url: r.url.clone(), size: r.size });
    let Some(rel) = rel.filter(|r| r.tag == version) else {
        return Err((400, format!("{version} is not the newer release found at start")));
    };
    if paused() {
        return Err((409, "a game is running: close it first".into()));
    }
    if UPDATING.swap(true, Ordering::SeqCst) {
        return Err((409, "an update is already running".into()));
    }
    // Sent already: the new copy is about to take over.
    if sent_recently() {
        UPDATING.store(false, Ordering::SeqCst);
        return Err((409, "an update is already running".into()));
    }
    // UPDATING is set, so holding() keeps new passes off from here; then no pass may be running.
    let r = if pass_idle() {
        download_and_send(&rel, paused)
    } else {
        Err((409, "a scan or build is in progress; try again in a moment".into()))
    };
    if r.is_ok() {
        HOLD_UNTIL.store(crate::sys::uptime_ms() + HOLD_MS, Ordering::SeqCst);
    }
    UPDATING.store(false, Ordering::SeqCst);
    match &r {
        Ok(()) => log!("update: {} ({} bytes) sent to elfldr", rel.tag, rel.size),
        Err((code, e)) => log!("update: {} failed ({code}): {e}", rel.tag),
    }
    r
}

fn download_and_send(rel: &Release, paused: &dyn Fn() -> bool) -> Result<(), (u16, String)> {
    let addr = SocketAddr::from(ELFLDR);
    let connect = || {
        TcpStream::connect_timeout(&addr, Duration::from_secs(1)).map_err(|e| (503, format!("elfldr on {addr}: {e}")))
    };
    // Probe before the download; the probe connection is closed at once.
    drop(connect()?);
    if rel.size > MAX {
        return Err((502, format!("{} asset is {} bytes, over {MAX}", rel.tag, rel.size)));
    }
    // ponytail: one try: the person waits for the answer in the UI and can press Update again.
    let b = get_retry("update", &rel.url, MAX, 1).map_err(|e| (502, format!("GET {}: {e}", rel.url)))?;
    if b.len() != rel.size || !is_module(&b) {
        let magic = &b[..b.len().min(4)];
        return Err((502, format!("{}: {} bytes (asset size {}), magic {magic:02x?}: not sent", rel.url, b.len(), rel.size)));
    }
    if paused() {
        return Err((409, "a game started during the download: close it, then update again".into()));
    }
    let mut s = connect()?;
    // A stalled elfldr must not hang this request (and UPDATING) for good.
    let _ = s.set_write_timeout(Some(Duration::from_secs(30)));
    s.write_all(&b)
        .and_then(|()| s.shutdown(Shutdown::Write))
        .map_err(|e| (502, format!("send {} bytes to elfldr on {addr}: {e}", b.len())))
}
