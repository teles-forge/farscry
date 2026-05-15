// Single global daemon — one process per machine, all terminals share it.
//
// Architecture:
//   ~/.farscry/daemon.pid   — PID of the running daemon (for liveness check)
//   ~/.farscry/daemon.sock  — Unix socket for IPC
//
// Line-based text protocol (client → daemon):
//   REGISTER <shell_pid>\n
//   UNREGISTER <shell_pid>\n
//   PING\n
//
// Daemon → client:
//   OK <window_id> <session_file_path>\n  (REGISTER response)
//   OK\n                                  (UNREGISTER / PING)
//   ERR <reason>\n
//
// Memory model:
//   - Zero OCR, zero model loading.
//   - pHash computed via IOSurface (zero heap allocation for pixels).
//   - Steady-state RSS: ~7 MB regardless of how many terminals register.
//   - Peak during capture: IOSurface lock is shared memory, not process heap.

use anyhow::{Context, Result};
use chrono::Utc;
use farscry_core::{vasf::VasfWriter, StateId};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(target_os = "macos")]
use crate::iosurface_phash as ios;
#[cfg(target_os = "macos")]
use crate::iosurface_phash::DisplayStream;

// ─── State ───────────────────────────────────────────────────────────────────

struct WindowEntry {
    #[allow(dead_code)]
    shell_pid: u32,
    last_hash: Option<StateId>,
    writer: VasfWriter,
    session_file: PathBuf,
}

type SharedState = Arc<Mutex<HashMap<u32, WindowEntry>>>;

// ─── Public entry points ──────────────────────────────────────────────────────

/// Start the global daemon.  Fails immediately if another daemon instance is
/// detected (PID file exists and process is alive).
pub fn run_daemon() -> Result<()> {
    let pid_path = pid_path();
    let sock_path = sock_path();

    if let Some(p) = pid_path.parent() {
        std::fs::create_dir_all(p)?;
    }

    evict_stale_daemon(&pid_path, &sock_path);

    std::fs::write(&pid_path, std::process::id().to_string())?;

    let listener =
        UnixListener::bind(&sock_path).context("another daemon instance may be running")?;

    let state: SharedState = Arc::new(Mutex::new(HashMap::new()));
    let state_cap = state.clone();

    let t_cap = thread::spawn(move || capture_loop(state_cap));

    eprintln!("[farscry:daemon] started pid={}", std::process::id());

    for s in listener.incoming().flatten() {
        let st = state.clone();
        thread::spawn(move || {
            if let Err(e) = handle_client(s, st) {
                eprintln!("[farscry:daemon] client error: {e}");
            }
        });
    }

    t_cap.join().ok();
    let _ = std::fs::remove_file(&sock_path);
    let _ = std::fs::remove_file(&pid_path);
    Ok(())
}

/// Called by `farscry record --daemon --global`.
/// Ensures a daemon is running, registers the terminal's shell PID,
/// and prints the assigned session file path to stdout.
pub fn connect_and_register(shell_pid: u32) -> Result<()> {
    ensure_daemon_running()?;

    let sock_path = sock_path();
    let mut stream = UnixStream::connect(&sock_path)
        .context("could not connect to farscry daemon")?;

    let msg = format!("REGISTER {shell_pid}\n");
    stream.write_all(msg.as_bytes())?;

    let mut resp = String::new();
    BufReader::new(stream).read_line(&mut resp)?;

    if resp.starts_with("OK ") {
        let tail = resp.trim().trim_start_matches("OK ");
        let (_, file) = tail.split_once(' ').unwrap_or(("0", tail));
        println!("{file}");
        Ok(())
    } else {
        anyhow::bail!("daemon rejected registration: {resp}");
    }
}

/// Called by the shell EXIT trap (`farscry daemon unregister <pid>`).
pub fn unregister(shell_pid: u32) -> Result<()> {
    let sock_path = sock_path();
    let Ok(mut stream) = UnixStream::connect(&sock_path) else {
        return Ok(());
    };
    let msg = format!("UNREGISTER {shell_pid}\n");
    stream.write_all(msg.as_bytes()).ok();
    Ok(())
}

// ─── IPC handler ─────────────────────────────────────────────────────────────

fn handle_client(stream: UnixStream, state: SharedState) -> Result<()> {
    let mut writer = stream.try_clone()?;
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.trim().splitn(2, ' ').collect();
        match parts.as_slice() {
            ["REGISTER", pid_str] => {
                let shell_pid: u32 = pid_str.parse().unwrap_or(0);
                match register(shell_pid, &state) {
                    Ok((wid, path)) => {
                        let resp = format!("OK {wid} {}\n", path.display());
                        writer.write_all(resp.as_bytes())?;
                    }
                    Err(e) => {
                        let resp = format!("ERR {e}\n");
                        writer.write_all(resp.as_bytes())?;
                    }
                }
            }
            ["UNREGISTER", pid_str] => {
                let shell_pid: u32 = pid_str.parse().unwrap_or(0);
                drop_window(shell_pid, &state);
                writer.write_all(b"OK\n")?;
            }
            ["PING"] => {
                writer.write_all(b"OK\n")?;
            }
            _ => {
                writer.write_all(b"ERR unknown command\n")?;
            }
        }
    }
    Ok(())
}

fn register(shell_pid: u32, state: &SharedState) -> Result<(u32, PathBuf)> {
    #[cfg(target_os = "macos")]
    let window_id = ios::find_terminal_window(shell_pid).unwrap_or(0);
    #[cfg(not(target_os = "macos"))]
    let window_id: u32 = 0;

    let dir = {
        #[cfg(target_os = "macos")]
        { ios::sessions_dir() }
        #[cfg(not(target_os = "macos"))]
        { dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".farscry").join("sessions") }
    };
    std::fs::create_dir_all(&dir)?;
    let ts = Utc::now().format("%Y%m%d-%H%M%S");
    let file = dir.join(format!("{ts}-{shell_pid}.vasf"));
    let writer = VasfWriter::create(&file)?;

    let entry = WindowEntry {
        shell_pid,
        last_hash: None,
        writer,
        session_file: file.clone(),
    };

    state.lock().unwrap().insert(shell_pid, entry);
    eprintln!("[farscry:daemon] registered pid={shell_pid} window_hint={window_id} → {}", file.display());
    Ok((window_id, file))
}

fn drop_window(shell_pid: u32, state: &SharedState) {
    if let Some(mut entry) = state.lock().unwrap().remove(&shell_pid) {
        entry.writer.finalize().ok();
        eprintln!(
            "[farscry:daemon] unregistered pid={shell_pid} → {}",
            entry.session_file.display()
        );
    }
}

// ─── Capture loop ─────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn hamming(a: StateId, b: StateId) -> u8 {
    (a.to_bits() ^ b.to_bits()).count_ones() as u8
}

fn capture_loop(state: SharedState) {
    let threshold: u8 = 10;
    let mut idle_ticks: u32 = 0;

    // Start a single 32×32 CGDisplayStream — zero heap allocation for pixels.
    // The GPU scales the display before delivering frames to us.
    #[cfg(target_os = "macos")]
    let stream = DisplayStream::start(2); // up to 2 pHash/sec
    #[cfg(not(target_os = "macos"))]
    let stream: Option<()> = None;

    loop {
        thread::sleep(Duration::from_secs(1));

        let mut guard = state.lock().unwrap();
        if guard.is_empty() {
            idle_ticks += 1;
            if idle_ticks > 5 {
                eprintln!("[farscry:daemon] no windows registered, exiting");
                drop(guard);
                std::process::exit(0);
            }
            continue;
        }
        idle_ticks = 0;

        // Read the latest display pHash from the stream (zero copy).
        #[cfg(target_os = "macos")]
        let current_hash = stream.as_ref().and_then(|s| s.latest_phash());
        #[cfg(not(target_os = "macos"))]
        let current_hash: Option<StateId> = None;

        let Some(hash) = current_hash else {
            continue;
        };
        let ts = now_ms();

        // All registered terminals share the same display-level pHash.
        for entry in guard.values_mut() {
            let is_new = entry
                .last_hash
                .map(|prev| hamming(hash, prev) > threshold)
                .unwrap_or(true);

            if is_new {
                entry.writer.append_state(hash, "").ok();
                entry.last_hash = Some(hash);
            } else {
                entry.writer.append_timeline(ts, hash).ok();
            }
        }
    }
}

// ─── Daemon lifecycle helpers ─────────────────────────────────────────────────

fn evict_stale_daemon(pid_path: &PathBuf, sock_path: &PathBuf) {
    if let Ok(s) = std::fs::read_to_string(pid_path) {
        let pid: u32 = s.trim().parse().unwrap_or(0);
        let alive = {
            #[cfg(target_os = "macos")]
            { ios::process_alive(pid) }
            #[cfg(not(target_os = "macos"))]
            { pid > 0 }
        };
        if !alive {
            let _ = std::fs::remove_file(sock_path);
            let _ = std::fs::remove_file(pid_path);
        }
    }
}

fn sock_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    { ios::daemon_sock_file() }
    #[cfg(not(target_os = "macos"))]
    { dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".farscry").join("daemon.sock") }
}

fn pid_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    { ios::daemon_pid_file() }
    #[cfg(not(target_os = "macos"))]
    { dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".farscry").join("daemon.pid") }
}

fn ensure_daemon_running() -> Result<()> {
    let sock_path = sock_path();

    // Fast path: socket exists and daemon answers PING
    if sock_path.exists() {
        if let Ok(mut s) = UnixStream::connect(&sock_path) {
            if s.write_all(b"PING\n").is_ok() {
                let mut resp = String::new();
                if BufReader::new(s).read_line(&mut resp).is_ok() && resp.starts_with("OK") {
                    return Ok(());
                }
            }
        }
        // Stale socket — clean up
        let _ = std::fs::remove_file(&sock_path);
    }

    // Start daemon
    let exe = std::env::current_exe()?;
    std::process::Command::new(&exe)
        .args(["daemon", "--start"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to start daemon")?;

    // Wait up to 5 s for socket to appear
    for _ in 0..50 {
        thread::sleep(Duration::from_millis(100));
        if sock_path.exists() {
            if let Ok(mut s) = UnixStream::connect(&sock_path) {
                s.write_all(b"PING\n").ok();
                let mut resp = String::new();
                if BufReader::new(s).read_line(&mut resp).is_ok() && resp.starts_with("OK") {
                    return Ok(());
                }
            }
        }
    }

    anyhow::bail!("daemon did not become ready within 5 s")
}
