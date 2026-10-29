//! Per-PTY native TUI submissions, consumed without retaining a second transcript.
use super::*;
use std::fs::File;
use std::io;
use std::os::windows::fs::FileExt as _;
use std::path::Path;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Ioctl::{
    FILE_ZERO_DATA_INFORMATION, FSCTL_SET_SPARSE, FSCTL_SET_ZERO_DATA,
};

const PREFIX: &str = ".pebrel-codex-tui-";
const LINE_LIMIT: usize = 128 * 1024;
const READ_LIMIT: usize = 256 * 1024;
const BACKLOG_LIMIT: u64 = 4 * 1024 * 1024;

pub(crate) struct Capture {
    file: File,
    _path: tempfile::TempPath,
    identity: (u32, u32, u32),
    owner: Option<(u32, u64)>,
    offset: u64,
    pending: Vec<u8>,
    discard_line: bool,
    client: Option<String>,
}

impl Capture {
    #[cfg(all(debug_assertions, feature = "gpui-test-support"))]
    pub(crate) fn qa_path(&self) -> &Path {
        &self._path
    }

    pub(crate) fn prepare(
        env: &mut HashMap<String, String>,
    ) -> io::Result<Option<Arc<Mutex<Self>>>> {
        // Existing explicit recording belongs to the user, including an explicit opt-out.
        if ["CODEX_TUI_RECORD_SESSION", "CODEX_TUI_SESSION_LOG_PATH"]
            .iter()
            .any(|key| env.contains_key(*key) || std::env::var_os(key).is_some())
        {
            return Ok(None);
        }
        let named = tempfile::Builder::new()
            .prefix(PREFIX)
            .suffix(".jsonl")
            .make(crate::platform::private_file::create_ephemeral)?;
        let path = named.path().to_string_lossy().into_owned();
        let (file, guard) = named.into_parts();
        ioctl(&file, FSCTL_SET_SPARSE, &[])?;
        let identity = file_identity(file.as_raw_handle()).ok_or_else(io::Error::last_os_error)?;
        env.insert("CODEX_TUI_RECORD_SESSION".into(), "1".into());
        env.insert("CODEX_TUI_SESSION_LOG_PATH".into(), path);
        Ok(Some(Arc::new(Mutex::new(Self {
            file,
            _path: guard,
            identity,
            owner: None,
            offset: 0,
            pending: Vec::new(),
            discard_line: false,
            client: None,
        }))))
    }

    fn reset(&mut self) {
        self.offset = 0;
        self.pending.clear();
        self.discard_line = false;
        self.client = None;
    }

    fn consume(&mut self) -> io::Result<()> {
        let end = self.file.metadata()?.len();
        if end < self.offset {
            self.reset();
        }
        if end == self.offset {
            return Ok(());
        }
        if end.saturating_sub(self.offset) > BACKLOG_LIMIT {
            self.reset();
            self.offset = end;
            self.discard_line = true;
            return self.erase(end);
        }
        let mut bytes = vec![0; (end - self.offset).min(READ_LIMIT as u64) as usize];
        let count = self.file.seek_read(&mut bytes, self.offset)?;
        self.offset += count as u64;
        self.ingest(&bytes[..count]);
        // Sparse zeroing never seeks or truncates the TUI's independently opened writer.
        self.erase(self.offset)
    }

    fn erase(&self, end: u64) -> io::Result<()> {
        let range = FILE_ZERO_DATA_INFORMATION {
            FileOffset: 0,
            BeyondFinalZero: i64::try_from(end)
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (&range as *const FILE_ZERO_DATA_INFORMATION).cast::<u8>(),
                std::mem::size_of_val(&range),
            )
        };
        ioctl(&self.file, FSCTL_SET_ZERO_DATA, bytes)
    }

    fn ingest(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if byte == b'\n' {
                if !self.discard_line {
                    self.record();
                }
                self.pending.clear();
                self.discard_line = false;
            } else if !self.discard_line {
                if self.pending.len() >= LINE_LIMIT {
                    self.pending.clear();
                    self.discard_line = true;
                    self.client = None;
                } else {
                    self.pending.push(byte);
                }
            }
        }
    }

    fn record(&mut self) {
        let Ok(record) = serde_json::from_slice::<serde_json::Value>(&self.pending) else {
            return;
        };
        match (record["dir"].as_str(), record["kind"].as_str()) {
            (Some("meta"), Some("session_start" | "session_end"))
            | (Some("to_tui"), Some("new_session" | "clear_ui")) => self.client = None,
            (Some("to_tui"), Some("app_event"))
                if matches!(
                    record["variant"].as_str(),
                    Some(
                        "OpenResumePicker"
                            | "ResumeSessionByIdOrName"
                            | "SelectAgentThread"
                            | "SelectAgentsOverviewThread"
                            | "ForkCurrentSession"
                            | "ForkAgentsOverviewThread"
                            | "ForkAgentsOverviewThreadReady"
                            | "ClearUiAndSubmitUserMessage"
                    )
                ) =>
            {
                self.client = None;
            },
            (Some("from_tui"), Some("op")) => {
                if let Some(id) = record["payload"]["UserTurn"]["client_user_message_id"]
                    .as_str()
                    .filter(|id| valid_uuid(id))
                {
                    self.client = Some(id.to_owned());
                }
            },
            _ => {},
        }
    }

    fn resolve(&mut self, records: &str) -> Option<CodexProbeUpdate> {
        let records = records
            .lines()
            .filter_map(parse_probe_record)
            .filter_map(|record| {
                let metadata =
                    serde_json::from_str::<serde_json::Value>(&record.first_line).ok()?;
                Some((record, metadata))
            })
            .collect::<Vec<_>>();
        let mut owners = Vec::new();
        for (record, meta) in &records {
            if meta["type"] == "pebrel_tui_capture"
                && serde_json::from_value::<(u32, u32, u32)>(meta["file_id"].clone()).ok()
                    == Some(self.identity)
                && let Some(created) = meta["created"].as_u64().filter(|value| *value != 0)
            {
                let owner = (record._pid, created);
                if !owners.contains(&owner) {
                    owners.push(owner);
                }
            }
        }
        if owners.len() != 1 {
            if self.owner.take().is_some() {
                self.reset();
                return Some(CodexProbeUpdate::Clear);
            }
            return None;
        }
        if self.owner != Some(owners[0]) {
            self.reset();
            self.owner = Some(owners[0]);
        }
        if self.consume().is_err() {
            self.client = None;
        }
        let Some(client) = self.client.as_deref() else {
            return Some(CodexProbeUpdate::Clear);
        };
        let mut roots = Vec::new();
        for (record, meta) in &records {
            if record._pid == owners[0].0
                && meta["type"] == "pebrel_tui_database"
                && meta["created"].as_u64() == Some(owners[0].1)
                && let Some(root) = Path::new(&record.link).parent()
                && !roots.contains(&root)
            {
                roots.push(root);
            }
        }
        if roots.len() != 1 {
            return Some(CodexProbeUpdate::Clear);
        }
        Some(match accepted_session(roots[0], client) {
            Some(session) => {
                CodexProbeUpdate::Session { session, submission_id: Some(client.to_owned()) }
            },
            None => CodexProbeUpdate::Clear,
        })
    }
}

fn ioctl(file: &File, code: u32, input: &[u8]) -> io::Result<()> {
    let mut written = 0;
    let result = unsafe {
        DeviceIoControl(
            file.as_raw_handle(),
            code,
            if input.is_empty() { ptr::null() } else { input.as_ptr().cast() },
            input.len() as u32,
            ptr::null_mut(),
            0,
            &mut written,
            ptr::null_mut(),
        )
    };
    if result == 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

pub(super) fn handle_metadata(
    path: &str,
    handle: HANDLE,
    created: u64,
) -> Option<serde_json::Value> {
    let name = Path::new(path).file_name()?.to_str()?;
    if name.starts_with(PREFIX) && name.ends_with(".jsonl") {
        Some(serde_json::json!({
            "type":"pebrel_tui_capture","file_id":file_identity(handle)?,"created":created,
        }))
    } else if matches!(name, "thread_history_1.sqlite" | "state_5.sqlite" | "logs_2.sqlite") {
        Some(serde_json::json!({"type":"pebrel_tui_database","created":created}))
    } else {
        None
    }
}

fn readonly(path: &Path, deadline: Instant) -> Option<rusqlite::Connection> {
    let db = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    db.busy_timeout(Duration::from_millis(50)).ok()?;
    db.progress_handler(1000, Some(move || Instant::now() >= deadline)).ok()?;
    Some(db)
}

fn accepted_session(root: &Path, client: &str) -> Option<CodexSession> {
    let deadline = Instant::now() + Duration::from_secs(1);
    let history = readonly(&root.join("thread_history_1.sqlite"), deadline)?;
    // This is exact native ID correlation, not a latest-file/text/time heuristic.
    // Bound the source rows before filtering and reject copied IDs in distinct forks.
    let mut statement = history
        .prepare(
            "SELECT DISTINCT thread_id FROM
             (SELECT thread_id,item_type,item_json FROM thread_items ORDER BY rowid DESC LIMIT 4096)
             WHERE item_type='userMessage' AND length(item_json)<=131072
             AND json_extract(item_json,'$.clientId')=?1 LIMIT 2",
        )
        .ok()?;
    let threads = statement
        .query_map([client], |row| row.get::<_, String>(0))
        .ok()?
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if threads.len() != 1 || !valid_uuid(&threads[0]) {
        return None;
    }
    let state = readonly(&root.join("state_5.sqlite"), deadline)?;
    let path: String = state
        .query_row("SELECT rollout_path FROM threads WHERE id=?1", [&threads[0]], |row| row.get(0))
        .ok()?;
    if !crate::session::valid_native_session_file(&path) {
        return None;
    }
    let header = read_metadata_line(File::open(&path).ok()?).ok()?;
    let session = parse_probe_context(&format!("1\t0\t{path}\t{header}"))?;
    (session.session_id == threads[0]).then_some(session)
}

pub(crate) fn probe(shell_pid: u32, capture: Arc<Mutex<Capture>>) -> Option<CodexProbeUpdate> {
    let shell = open_process(shell_pid, PROCESS_QUERY_INFORMATION)?;
    let output = worker::probe_output(shell_pid, creation_time(shell.0)?)?;
    capture.lock().ok()?.resolve(&output)
}

#[cfg(test)]
#[path = "tui_capture_tests.rs"]
mod tests;
