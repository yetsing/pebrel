use super::*;
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;

const CLIENT: &str = "20923285-eccd-4182-8355-b04c2df29bc1";
const THREAD: &str = "01a18322-c174-71f0-9654-a8cb1370fbd1";

fn capture() -> Arc<Mutex<Capture>> {
    Capture::prepare(&mut HashMap::new()).unwrap().unwrap()
}

fn native_db(root: &Path, client: &str, thread: &str, source: &str) -> PathBuf {
    let history = rusqlite::Connection::open(root.join("thread_history_1.sqlite")).unwrap();
    history
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS thread_items(thread_id TEXT,item_type TEXT,item_json TEXT)",
        )
        .unwrap();
    history.execute("INSERT INTO thread_items VALUES(?1,'userMessage',?2)", rusqlite::params![thread,json!({"type":"userMessage","clientId":client,"content":[{"type":"text","text":"accepted body"}]}).to_string()]).unwrap();
    let rollout = root.join(format!("rollout-2026-10-29T00-00-00-{thread}.jsonl"));
    std::fs::write(&rollout, format!("{}\n", json!({"type":"session_meta","payload":{"id":thread,"source":source,"thread_source":"user","cwd":root}}))).unwrap();
    let state = rusqlite::Connection::open(root.join("state_5.sqlite")).unwrap();
    state
        .execute_batch("CREATE TABLE IF NOT EXISTS threads(id TEXT PRIMARY KEY,rollout_path TEXT)")
        .unwrap();
    state
        .execute(
            "INSERT OR REPLACE INTO threads VALUES(?1,?2)",
            rusqlite::params![thread, rollout.to_str().unwrap()],
        )
        .unwrap();
    rollout
}

#[test]
fn native_submission_id_requires_one_accepted_main_thread_and_matching_header() {
    let root = tempfile::tempdir().unwrap();
    let rollout = native_db(root.path(), CLIENT, THREAD, "cli");
    assert!(accepted_session(root.path(), "00000000-0000-0000-0000-000000000000").is_none());
    assert_eq!(
        accepted_session(root.path(), CLIENT).unwrap().session_file,
        rollout.to_string_lossy()
    );
    std::fs::write(
        &rollout,
        format!("{}\n", json!({"type":"session_meta","payload":{"id":THREAD,"source":"subagent"}})),
    )
    .unwrap();
    assert!(accepted_session(root.path(), CLIENT).is_none());
    native_db(root.path(), CLIENT, "01a18322-c174-71f0-9654-a8cb1370fbd2", "cli");
    assert!(
        accepted_session(root.path(), CLIENT).is_none(),
        "a fork copying the client ID is ambiguous"
    );
}

#[test]
fn native_tui_accepts_inherited_vscode_originator_but_still_rejects_subagents() {
    let root = tempfile::tempdir().unwrap();
    native_db(root.path(), CLIENT, THREAD, "vscode");
    assert_eq!(accepted_session(root.path(), CLIENT).unwrap().session_id, THREAD);
    let path = root.path().join(format!("rollout-2026-10-29T00-00-00-{THREAD}.jsonl"));
    std::fs::write(
        path,
        format!(
            "{}\n",
            json!({"type":"session_meta","payload":{
                "id":THREAD,"source":"vscode","thread_source":"subagent",
            }})
        ),
    )
    .unwrap();
    assert!(accepted_session(root.path(), CLIENT).is_none());
}

#[test]
fn capture_only_keeps_submission_ids_and_explicit_navigation_clears_them() {
    let source = capture();
    let mut source = source.lock().unwrap();
    let turn = format!(
        "{}\n",
        json!({"dir":"from_tui","kind":"op","payload":{"UserTurn":{"client_user_message_id":CLIENT,"items":[{"text":"never persist this private draft"}]}}})
    );
    for byte in turn.as_bytes() {
        source.ingest(&[*byte]);
    }
    assert_eq!(source.client.as_deref(), Some(CLIENT));
    assert!(source.pending.is_empty());
    source.ingest(b"{\"dir\":\"to_tui\",\"kind\":\"new_session\"}\n");
    assert!(source.client.is_none());
    source.ingest(turn.as_bytes());
    source.ingest(&vec![b'x'; LINE_LIMIT + 1]);
    assert!(source.client.is_none());
    assert!(source.pending.is_empty());
    source.ingest(b"\n");
    source.ingest(turn.as_bytes());
    assert_eq!(source.client.as_deref(), Some(CLIENT));
}

#[test]
fn capture_erases_consumed_text_and_os_removes_it_after_the_last_handle_closes() {
    let source = capture();
    let path = source.lock().unwrap()._path.to_path_buf();
    let mut writer = OpenOptions::new().write(true).open(&path).unwrap();
    let input = format!(
        "{}\n",
        json!({"dir":"from_tui","kind":"op","payload":{"UserTurn":{"client_user_message_id":CLIENT,"items":[{"text":"private body"}]}}})
    );
    writer.write_all(input.as_bytes()).unwrap();
    source.lock().unwrap().consume().unwrap();
    assert!(std::fs::read(&path).unwrap().iter().all(|byte| *byte == 0));
    assert_eq!(source.lock().unwrap().client.as_deref(), Some(CLIENT));
    drop(source);
    drop(writer);
    assert!(!path.exists());
}

#[test]
fn only_matching_live_handle_identity_and_one_owner_can_bind_the_capture() {
    let source = capture();
    let mut source = source.lock().unwrap();
    let root = tempfile::tempdir().unwrap();
    native_db(root.path(), CLIENT, THREAD, "cli");
    let path = source._path.to_path_buf();
    let turn = format!(
        "{}\n",
        json!({"dir":"from_tui","kind":"op","payload":{"UserTurn":{"client_user_message_id":CLIENT}}})
    );
    OpenOptions::new().write(true).open(&path).unwrap().write_all(turn.as_bytes()).unwrap();
    let record = |pid, created, identity| {
        format!(
            "{pid}\t0\t{}\t{}\n",
            path.display(),
            json!({"type":"pebrel_tui_capture","file_id":identity,"created":created})
        )
    };
    assert!(source.resolve(&record(4, 10, (0, 0, 0))).is_none());
    let owner = record(4, 10, source.identity);
    let db = format!(
        "4\t0\t{}\t{}\n",
        root.path().join("state_5.sqlite").display(),
        json!({"type":"pebrel_tui_database","created":10})
    );
    assert!(
        matches!(source.resolve(&format!("{owner}{db}")),Some(CodexProbeUpdate::Session {session, submission_id}) if session.session_id==THREAD && submission_id.as_deref()==Some(CLIENT))
    );
    let other = record(5, 11, source.identity);
    assert!(matches!(
        source.resolve(&format!("{owner}{other}{db}")),
        Some(CodexProbeUpdate::Clear)
    ));
}

#[test]
fn native_capture_follows_the_verified_tui_handle_instead_of_the_daemon_file_owner() {
    use super::super::tests::Fixture;
    let executable_dir =
        tempfile::tempdir_in(std::env::current_exe().unwrap().parent().unwrap()).unwrap();
    let executable = executable_dir.path().join("codex.exe");
    std::fs::hard_link(std::env::current_exe().unwrap(), &executable).unwrap();
    let database = tempfile::tempdir().unwrap();
    native_db(database.path(), CLIENT, THREAD, "cli");
    let source = capture();
    let path = source.lock().unwrap()._path.to_path_buf();
    let turn = format!(
        "{}\n",
        json!({"dir":"from_tui","kind":"op","payload":{"UserTurn":{"client_user_message_id":CLIENT}}})
    );
    OpenOptions::new().write(true).open(&path).unwrap().write_all(turn.as_bytes()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let fixture = Fixture::launch(
        root.path(),
        &executable,
        &[&path, &database.path().join("state_5.sqlite")],
        false,
    );
    let foreign = tempfile::tempdir().unwrap();
    let unrelated = Fixture::launch(
        foreign.path(),
        &executable,
        &[&database.path().join("state_5.sqlite")],
        false,
    );
    assert!(probe(unrelated.shell_pid(), source.clone()).is_none());
    assert!(
        matches!(probe(fixture.shell_pid(), source.clone()), Some(CodexProbeUpdate::Session {session, submission_id}) if session.session_id == THREAD && submission_id.as_deref()==Some(CLIENT))
    );
    fixture.finish();
    unrelated.finish();
    assert!(matches!(probe(std::process::id(), source), Some(CodexProbeUpdate::Clear)));
}
