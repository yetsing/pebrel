//! Exact hook identity selects the main wire; no cwd or newest-file discovery.

use super::*;

fn records(capture: &Capture) -> impl DoubleEndedIterator<Item = Value> + '_ {
    let bytes = if capture.start == 0 {
        capture.bytes.as_slice()
    } else {
        capture
            .bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|end| &capture.bytes[end + 1..])
            .unwrap_or_default()
    };
    bytes
        .split_inclusive(|byte| *byte == b'\n')
        .filter(|line| line.ends_with(b"\n"))
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
}

pub(crate) fn latest_user(home: &Path, session: &str) -> Option<String> {
    let mut before = None;
    let mut selected = None;
    for _ in 0..8 {
        let capture = capture_file(&home.join("session_index.jsonl"), before).ok()?;
        if let Some(entry) = records(&capture).rev().find(|entry| entry["sessionId"] == session) {
            selected = entry["sessionDir"].as_str().map(std::path::PathBuf::from);
            break;
        }
        if capture.start == 0 {
            break;
        }
        before = Some(capture.start);
    }
    let directory = selected?.canonicalize().ok()?;
    if !directory.starts_with(home.join("sessions").canonicalize().ok()?) {
        return None;
    }
    let mut state = Vec::new();
    File::open(directory.join("state.json")).ok()?.take(64 * 1024).read_to_end(&mut state).ok()?;
    if serde_json::from_slice::<Value>(&state).ok()?["id"] != session {
        return None;
    }
    let path = directory.join("agents/main/wire.jsonl");
    let mut before = None;
    let mut stamp = None;
    for _ in 0..8 {
        let capture = capture_file(&path, before).ok()?;
        if stamp.as_ref().is_some_and(|stamp| stamp != &capture.stamp) {
            return None;
        }
        stamp = Some(capture.stamp.clone());
        if let Some(record) = records(&capture).rev().find(|record| {
            record["type"] == "turn.prompt"
                && record["agentId"] == "main"
                && record["origin"]["kind"] == "user"
        }) {
            return Some(text_content(&record["input"]));
        }
        if capture.start == 0 {
            break;
        }
        before = Some(capture.start);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn exact_kimi_identity_reads_only_main_user_turns_and_complete_records() {
        let home = tempfile::tempdir().unwrap();
        let directory = home.path().join("sessions/project/session_native");
        std::fs::create_dir_all(directory.join("agents/main")).unwrap();
        std::fs::write(
            home.path().join("session_index.jsonl"),
            format!(
                "{}\n",
                json!({
                    "sessionId":"session_native", "sessionDir":directory,
                })
            ),
        )
        .unwrap();
        std::fs::write(directory.join("state.json"), r#"{"id":"session_native"}"#).unwrap();
        let mut file = File::create(directory.join("agents/main/wire.jsonl")).unwrap();
        for record in [
            json!({"type":"turn.prompt","agentId":"main","origin":{"kind":"user"},"input":[{"type":"text","text":"真正\n用户消息"}]}),
            json!({"type":"turn.prompt","agentId":"child","origin":{"kind":"user"},"input":[{"type":"text","text":"subagent"}]}),
            json!({"type":"turn.prompt","agentId":"main","origin":{"kind":"system_trigger"},"input":[{"type":"text","text":"automatic continuation"}]}),
        ] {
            writeln!(file, "{record}").unwrap();
        }
        write!(file, "{{\"type\":\"turn.prompt\"").unwrap();
        assert_eq!(latest_user(home.path(), "session_native").as_deref(), Some("真正\n用户消息"));
        assert!(latest_user(home.path(), "same-directory-other-session").is_none());
        std::fs::write(directory.join("state.json"), r#"{"id":"replaced"}"#).unwrap();
        assert!(latest_user(home.path(), "session_native").is_none());
    }
}
