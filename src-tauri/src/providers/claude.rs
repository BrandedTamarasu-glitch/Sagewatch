use super::*;
use crate::domain::{
    AllowanceWindow, Freshness, Health, Provider, ProviderStatus, Source, SourceConfidence,
    WindowKind,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tokio::{process::Command, sync::Mutex};

const CLAUDE_STARTUP_DELAY: Duration = Duration::from_secs(3);
const SNAPSHOT_WAIT: Duration = Duration::from_secs(17);
const USAGE_MODAL_WAIT: Duration = Duration::from_secs(12);
const EXIT_WAIT: Duration = Duration::from_secs(1);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const VERSION_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_SCANNER_BYTES: usize = 16 * 1024;
const USAGE_COMMAND: &[u8] = b"/usage\r";
const ESCAPE_COMMAND: &[u8] = b"\x1b";
const EXIT_COMMAND: &[u8] = b"/exit\r";
const TRUST_CONFIRM_COMMAND: &[u8] = b"\r";

#[derive(Clone, Debug)]
pub struct ClaudeAdapter {
    path: PathBuf,
    stale_after: Duration,
    executable: Option<PathBuf>,
    work_dir: PathBuf,
    usage_probe_enabled: Arc<AtomicBool>,
    probe_lock: Arc<Mutex<()>>,
}
impl ClaudeAdapter {
    pub fn new(path: impl Into<PathBuf>, stale_after: Duration) -> Self {
        let path = path.into();
        let work_dir = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("probe-work");
        Self {
            path,
            stale_after,
            executable: discover_executable("claude"),
            work_dir,
            usage_probe_enabled: Arc::new(AtomicBool::new(false)),
            probe_lock: Arc::new(Mutex::new(())),
        }
    }

    #[cfg(test)]
    fn with_probe_executable(mut self, executable: impl Into<PathBuf>) -> Self {
        self.executable = Some(executable.into());
        self
    }
}
#[derive(Deserialize)]
struct Snapshot {
    observed_at: DateTime<Utc>,
    #[serde(default)]
    plan: Option<String>,
    rate_limits: RateLimits,
}
#[derive(Deserialize)]
struct RateLimits {
    five_hour: Option<Window>,
    seven_day: Option<Window>,
    seven_day_sonnet: Option<Window>,
    seven_day_opus: Option<Window>,
    extra_usage: Option<Window>,
    #[serde(default)]
    models: BTreeMap<String, Window>,
}
#[derive(Deserialize)]
struct Window {
    #[serde(default, alias = "used_percentage")]
    utilization: Option<f64>,
    #[serde(default)]
    remaining_percentage: Option<f64>,
    #[serde(alias = "reset_at")]
    resets_at: Option<DateTime<Utc>>,
}
impl ClaudeAdapter {
    async fn version(&self) -> Result<String, AdapterError> {
        let executable = self
            .executable
            .as_ref()
            .ok_or_else(|| ae(AdapterErrorKind::Unavailable, true))?;
        let output = tokio::time::timeout(
            VERSION_TIMEOUT,
            Command::new(executable)
                .arg("--version")
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output(),
        )
        .await
        .map_err(|_| ae(AdapterErrorKind::Timeout, true))?
        .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
        if !output.status.success() || output.stdout.len() > 1024 {
            return Err(ae(AdapterErrorKind::Unavailable, true));
        }
        String::from_utf8(output.stdout)
            .map(|value| value.trim().to_owned())
            .map_err(|_| ae(AdapterErrorKind::SourceChanged, false))
    }

    fn compatible(version: &str) -> bool {
        version.split_whitespace().any(|part| {
            part.strip_prefix('v')
                .unwrap_or(part)
                .split_once('.')
                .is_some_and(|(major, rest)| major == "2" && rest.starts_with("1."))
        })
    }

    fn observed_at(path: &Path) -> Option<DateTime<Utc>> {
        let bytes = fs::read(path).ok()?;
        if bytes.len() > 64 * 1024 {
            return None;
        }
        serde_json::from_slice::<Snapshot>(&bytes)
            .ok()
            .map(|snapshot| snapshot.observed_at)
    }

    async fn run_usage_probe(&self) -> Result<(), AdapterError> {
        let _guard = self
            .probe_lock
            .try_lock()
            .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
        let version = self.version().await?;
        if !Self::compatible(&version) {
            return Err(ae(AdapterErrorKind::Unsupported, false));
        }
        let executable = self
            .executable
            .clone()
            .ok_or_else(|| ae(AdapterErrorKind::Unavailable, true))?;
        let snapshot_path = self.path.clone();
        let work_dir = self.work_dir.clone();
        tokio::time::timeout(
            Duration::from_secs(22),
            tokio::task::spawn_blocking(move || {
                run_probe_process(&executable, &snapshot_path, &work_dir)
            }),
        )
        .await
        .map_err(|_| ae(AdapterErrorKind::Timeout, true))?
        .map_err(|_| ae(AdapterErrorKind::Internal, false))?
    }

    fn load(&self) -> Result<ProviderStatus, AdapterError> {
        let metadata = fs::symlink_metadata(&self.path)
            .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 64 * 1024 {
            return Err(ae(AdapterErrorKind::InvalidData, false));
        }
        let bytes = fs::read(&self.path).map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
        let snapshot: Snapshot = serde_json::from_slice(&bytes)
            .map_err(|_| ae(AdapterErrorKind::SourceChanged, false))?;
        let mut windows = Vec::new();
        if let Some(w) = snapshot.rate_limits.five_hour {
            windows.push(cw(
                "five_hour",
                "Five hour",
                WindowKind::Rolling,
                Some(300),
                w,
            )?);
        }
        if let Some(w) = snapshot.rate_limits.seven_day {
            windows.push(cw(
                "seven_day",
                "Seven day",
                WindowKind::Weekly,
                Some(10080),
                w,
            )?);
        }
        if let Some(w) = snapshot.rate_limits.seven_day_sonnet {
            windows.push(cw(
                "seven_day_sonnet",
                "Sonnet seven day",
                WindowKind::ModelScoped,
                Some(10080),
                w,
            )?);
        }
        if let Some(w) = snapshot.rate_limits.seven_day_opus {
            windows.push(cw(
                "seven_day_opus",
                "Opus seven day",
                WindowKind::ModelScoped,
                Some(10080),
                w,
            )?);
        }
        if let Some(w) = snapshot.rate_limits.extra_usage {
            windows.push(cw(
                "extra_usage",
                "Extra usage",
                WindowKind::Credits,
                None,
                w,
            )?);
        }
        for (model, w) in snapshot.rate_limits.models {
            if model.is_empty() || model.len() > 128 {
                return Err(ae(AdapterErrorKind::InvalidData, false));
            }
            windows.push(cw(
                &format!("model:{model}"),
                &model,
                WindowKind::ModelScoped,
                None,
                w,
            )?);
        }
        if windows.is_empty() {
            return Err(ae(AdapterErrorKind::SourceChanged, false));
        }
        let age = Utc::now()
            .signed_duration_since(snapshot.observed_at)
            .to_std()
            .unwrap_or_default();
        Ok(ProviderStatus {
            schema_version: 1,
            provider: Provider::Claude,
            plan: snapshot.plan.unwrap_or_else(|| "unknown".into()),
            observed_at: snapshot.observed_at,
            last_successful_at: Some(snapshot.observed_at),
            source: Source::ClaudeStatusline,
            source_confidence: SourceConfidence::DocumentedLocal,
            freshness: if age > self.stale_after {
                Freshness::Stale
            } else {
                Freshness::Live
            },
            health: Health::Healthy,
            headline_window_id: None,
            windows,
        })
    }
}
fn cw(
    id: &str,
    label: &str,
    kind: WindowKind,
    duration: Option<u32>,
    w: Window,
) -> Result<AllowanceWindow, AdapterError> {
    if w.utilization.is_none() && w.remaining_percentage.is_none() {
        return Err(ae(AdapterErrorKind::SourceChanged, false));
    }
    if w.utilization
        .iter()
        .chain(w.remaining_percentage.iter())
        .any(|value| !value.is_finite())
    {
        return Err(ae(AdapterErrorKind::SourceChanged, false));
    }
    Ok(AllowanceWindow {
        id: id.into(),
        label: label.into(),
        duration_minutes: duration,
        used_percent: w.utilization,
        remaining_percent: w.remaining_percentage,
        reset_at: w.resets_at,
        kind,
        is_active: true,
    })
}
fn ae(kind: AdapterErrorKind, retryable: bool) -> AdapterError {
    AdapterError {
        kind,
        message: safe_summary(kind).into(),
        retryable,
    }
}
#[async_trait]
impl ProviderAdapter for ClaudeAdapter {
    fn set_preferences(&self, preferences: &crate::domain::Preferences) {
        self.usage_probe_enabled
            .store(preferences.claude_usage_probe_enabled, Ordering::Relaxed);
    }

    async fn probe(&self) -> CapabilityReport {
        if self.usage_probe_enabled.load(Ordering::Relaxed) {
            match self.version().await {
                Ok(version) => CapabilityReport {
                    available: true,
                    compatible: Self::compatible(&version),
                    detail: if Self::compatible(&version) {
                        "compatible experimental Claude /usage probe".into()
                    } else {
                        "unsupported Claude CLI version".into()
                    },
                },
                Err(_) => CapabilityReport {
                    available: false,
                    compatible: false,
                    detail: "Claude CLI unavailable".into(),
                },
            }
        } else {
            CapabilityReport {
                available: self.path.is_file(),
                compatible: true,
                detail: if self.path.is_file() {
                    "Claude statusline snapshot available".into()
                } else {
                    "Claude statusline snapshot unavailable".into()
                },
            }
        }
    }
    async fn refresh(&self) -> Result<ProviderStatus, AdapterError> {
        if self.usage_probe_enabled.load(Ordering::Relaxed) {
            self.run_usage_probe().await?;
        }
        self.load()
    }
    fn diagnose(&self, error: &AdapterError) -> ProviderDiagnostics {
        ProviderDiagnostics {
            provider: Provider::Claude,
            health: health_for_error(error.kind),
            summary: safe_summary(error.kind).into(),
            retryable: error.retryable,
        }
    }
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider: Provider::Claude,
            supports_refresh: true,
            session_attached: true,
        }
    }
}

fn discover_executable(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find_map(|candidate| {
            let canonical = candidate.canonicalize().ok()?;
            canonical.is_file().then_some(canonical)
        })
}

fn private_work_dir(path: &Path) -> Result<(), AdapterError> {
    fs::create_dir_all(path).map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
    }
    Ok(())
}

fn run_probe_process(
    executable: &Path,
    snapshot_path: &Path,
    work_dir: &Path,
) -> Result<(), AdapterError> {
    private_work_dir(work_dir)?;
    let before = ClaudeAdapter::observed_at(snapshot_path);
    let system = native_pty_system();
    let pair = system
        .openpty(PtySize {
            rows: 24,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
    let mut command = CommandBuilder::new(executable);
    command.cwd(work_dir);
    command.env("TERM", "xterm-256color");
    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|_| ae(AdapterErrorKind::Internal, false))?;
    let trust_prompt_seen = Arc::new(AtomicBool::new(false));
    let interactive_prompt_seen = Arc::new(AtomicBool::new(false));
    let usage_screen_seen = Arc::new(AtomicBool::new(false));
    let reader_trust_prompt_seen = Arc::clone(&trust_prompt_seen);
    let reader_interactive_prompt_seen = Arc::clone(&interactive_prompt_seen);
    let reader_usage_screen_seen = Arc::clone(&usage_screen_seen);
    let reader_thread = thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        let mut scanner = Vec::with_capacity(MAX_SCANNER_BYTES);
        loop {
            let read = reader.read(&mut buffer).unwrap_or(0);
            if read == 0 {
                break;
            }
            update_trust_scanner(&mut scanner, &buffer[..read]);
            if trust_prompt(&scanner) {
                reader_trust_prompt_seen.store(true, Ordering::Release);
            }
            if interactive_prompt(&scanner) {
                reader_interactive_prompt_seen.store(true, Ordering::Release);
            }
            if usage_screen(&scanner) {
                reader_usage_screen_seen.store(true, Ordering::Release);
            }
        }
    });
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|_| ae(AdapterErrorKind::Internal, false))?;

    let result = (|| {
        let startup_deadline = Instant::now() + CLAUDE_STARTUP_DELAY;
        while Instant::now() < startup_deadline {
            if trust_prompt_seen.load(Ordering::Acquire)
                || interactive_prompt_seen.load(Ordering::Acquire)
            {
                break;
            }
            thread::sleep(POLL_INTERVAL);
        }
        if trust_prompt_seen.load(Ordering::Acquire) {
            writer
                .write_all(TRUST_CONFIRM_COMMAND)
                .and_then(|_| writer.flush())
                .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
            let trusted_startup_deadline = Instant::now() + CLAUDE_STARTUP_DELAY;
            while Instant::now() < trusted_startup_deadline
                && !interactive_prompt_seen.load(Ordering::Acquire)
            {
                thread::sleep(POLL_INTERVAL);
            }
        }
        writer
            .write_all(USAGE_COMMAND)
            .and_then(|_| writer.flush())
            .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
        let usage_started = Instant::now();
        let mut escaped_usage = false;
        let deadline = Instant::now() + SNAPSHOT_WAIT;
        while Instant::now() < deadline {
            if child
                .try_wait()
                .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?
                .is_some()
            {
                return Err(ae(AdapterErrorKind::Unavailable, true));
            }
            if ClaudeAdapter::observed_at(snapshot_path)
                .is_some_and(|observed| before.is_none_or(|previous| observed > previous))
            {
                if !escaped_usage {
                    writer
                        .write_all(ESCAPE_COMMAND)
                        .and_then(|_| writer.flush())
                        .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
                }
                thread::sleep(POLL_INTERVAL);
                writer
                    .write_all(EXIT_COMMAND)
                    .and_then(|_| writer.flush())
                    .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
                let exit_deadline = Instant::now() + EXIT_WAIT;
                while Instant::now() < exit_deadline {
                    if child
                        .try_wait()
                        .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?
                        .is_some()
                    {
                        return Ok(());
                    }
                    thread::sleep(POLL_INTERVAL);
                }
                return Ok(());
            }
            if !escaped_usage && usage_started.elapsed() >= USAGE_MODAL_WAIT {
                writer
                    .write_all(ESCAPE_COMMAND)
                    .and_then(|_| writer.flush())
                    .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
                escaped_usage = true;
            }
            thread::sleep(POLL_INTERVAL);
        }
        Err(ae(
            if usage_screen_seen.load(Ordering::Acquire) {
                AdapterErrorKind::Timeout
            } else {
                AdapterErrorKind::SourceChanged
            },
            true,
        ))
    })();

    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
    drop(writer);
    drop(pair.master);
    let _ = reader_thread.join();
    result
}

fn update_trust_scanner(scanner: &mut Vec<u8>, incoming: &[u8]) {
    scanner.extend_from_slice(incoming);
    if scanner.len() > MAX_SCANNER_BYTES {
        scanner.drain(..scanner.len() - MAX_SCANNER_BYTES);
    }
}

fn trust_prompt(scanner: &[u8]) -> bool {
    const SAFETY: &[u8] = b"Quick safety check";
    const TRUST: &[u8] = b"Yes, I trust this folder";
    scanner.windows(SAFETY.len()).any(|window| window == SAFETY)
        && scanner.windows(TRUST.len()).any(|window| window == TRUST)
}

fn interactive_prompt(scanner: &[u8]) -> bool {
    const PROMPT: &[u8] = "❯".as_bytes();
    scanner.windows(PROMPT.len()).any(|window| window == PROMPT)
}

fn usage_screen(scanner: &[u8]) -> bool {
    contains(scanner, b"Settings") && contains(scanner, b"Stats")
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Preferences;

    #[test]
    fn usage_probe_is_opt_in_and_version_gated() {
        let adapter = ClaudeAdapter::new("missing.json", Duration::from_secs(60))
            .with_probe_executable("/definitely/missing/claude");
        assert!(!adapter.usage_probe_enabled.load(Ordering::Relaxed));
        adapter.set_preferences(&Preferences {
            claude_usage_probe_enabled: true,
            ..Preferences::default()
        });
        assert!(adapter.usage_probe_enabled.load(Ordering::Relaxed));
        assert!(ClaudeAdapter::compatible("2.1.226 (Claude Code)"));
        assert!(ClaudeAdapter::compatible("claude v2.1.0"));
        assert!(!ClaudeAdapter::compatible("2.2.0 (Claude Code)"));
        assert!(!ClaudeAdapter::compatible("1.9.9 (Claude Code)"));
    }

    #[test]
    fn probe_timeouts_and_command_sequence_are_bounded() {
        assert_eq!(CLAUDE_STARTUP_DELAY, Duration::from_secs(3));
        assert_eq!(SNAPSHOT_WAIT, Duration::from_secs(17));
        assert_eq!(USAGE_MODAL_WAIT, Duration::from_secs(12));
        assert_eq!(EXIT_WAIT, Duration::from_secs(1));
        assert_eq!(USAGE_COMMAND, &[47, 117, 115, 97, 103, 101, 13]);
        assert_eq!(ESCAPE_COMMAND, &[27]);
        assert_eq!(EXIT_COMMAND, &[47, 101, 120, 105, 116, 13]);
        assert_eq!(TRUST_CONFIRM_COMMAND, &[13]);
    }

    #[test]
    fn trust_prompt_scanner_is_exact_chunk_safe_and_bounded() {
        let mut scanner = Vec::new();
        update_trust_scanner(&mut scanner, b"Quick safety ");
        assert!(!trust_prompt(&scanner));
        update_trust_scanner(
            &mut scanner,
            b"check\r\n  1. Yes, I trust this folder (Enter)",
        );
        assert!(trust_prompt(&scanner));
        update_trust_scanner(&mut scanner, "\r\n❯ Try something".as_bytes());
        assert!(interactive_prompt(&scanner));

        let mut unrelated = Vec::new();
        update_trust_scanner(&mut unrelated, &vec![b'x'; MAX_SCANNER_BYTES + 1_000]);
        assert_eq!(unrelated.len(), MAX_SCANNER_BYTES);
        assert!(!trust_prompt(&unrelated));
    }
    #[test]
    fn reads_sanitized_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("status.json");
        fs::write(&path,format!(r#"{{"observed_at":"{}","plan":"team","rate_limits":{{"five_hour":{{"utilization":20,"resets_at":null}},"seven_day":{{"utilization":80,"resets_at":null}},"models":{{"sonnet":{{"utilization":50,"resets_at":null}}}}}}}}"#,Utc::now().to_rfc3339())).unwrap();
        let status = ClaudeAdapter::new(path, Duration::from_secs(60))
            .load()
            .unwrap()
            .normalize()
            .unwrap();
        assert_eq!(status.windows.len(), 3);
        assert_eq!(status.headline_window_id.as_deref(), Some("seven_day"));
    }

    #[test]
    fn reads_shared_bridge_model_contract_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/claude-model-statusline.json");
        let status = ClaudeAdapter::new(path, Duration::from_secs(60))
            .load()
            .unwrap()
            .normalize()
            .unwrap();
        assert_eq!(status.plan, "Claude Team");
        assert_eq!(status.windows.len(), 1);
        assert_eq!(status.windows[0].id, "model:sonnet-5");
        assert_eq!(status.windows[0].remaining_percent, Some(42.0));
        assert_eq!(status.windows[0].kind, WindowKind::ModelScoped);
    }

    #[test]
    #[ignore = "requires a live Claude Code status-line observation"]
    fn live_statusline_snapshot_satisfies_contract() {
        let path = std::env::var_os("SAGEWATCH_CLAUDE_SNAPSHOT_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let data_home = std::env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(|home| PathBuf::from(home).join(".local/share"))
                    })
                    .expect("HOME or XDG_DATA_HOME is required for the live Claude test");
                data_home.join("sagewatch/ingest/claude-statusline.json")
            });
        let status = ClaudeAdapter::new(path, Duration::from_secs(10 * 60))
            .load()
            .expect("live Claude status-line snapshot should load")
            .normalize()
            .expect("live Claude status-line snapshot should satisfy the domain contract");
        assert_eq!(status.provider, Provider::Claude);
        assert!(!status.windows.is_empty());
    }

    #[tokio::test]
    #[ignore = "launches the user's authenticated Claude CLI in a PTY"]
    async fn live_usage_probe_advances_sanitized_snapshot() {
        let path = std::env::var_os("SAGEWATCH_CLAUDE_SNAPSHOT_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let data_home = std::env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(|home| PathBuf::from(home).join(".local/share"))
                    })
                    .expect("HOME or XDG_DATA_HOME is required for the live Claude test");
                data_home.join("sagewatch/ingest/claude-statusline.json")
            });
        let adapter = ClaudeAdapter::new(path, Duration::from_secs(10 * 60));
        adapter.set_preferences(&Preferences {
            claude_usage_probe_enabled: true,
            ..Preferences::default()
        });
        let status = adapter
            .refresh()
            .await
            .expect("/usage should advance the sanitized status-line snapshot");
        assert_eq!(status.provider, Provider::Claude);
        assert!(!status.windows.is_empty());
    }
}
