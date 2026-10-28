//! Volatile preview ownership; drafts and late reads cannot publish here.

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Owner {
    Shell,
    Agent { kind: String, session: Option<String>, epoch: u64 },
}

/// A native queue footer can invalidate content, never authorize new content.
pub(super) fn kimi_queue_present(screen: &str) -> bool {
    let mut queued_row = false;
    for line in screen.lines() {
        let line = line.trim();
        if line.starts_with("❯ ") {
            queued_row = true;
        } else if queued_row && line.starts_with("↑ to edit") {
            return true;
        }
    }
    false
}

pub(super) fn opencode_home_present(screen: &str) -> bool {
    screen.lines().any(|line| line.contains("█▀▀█ █▀▀█ █▀▀█ █▀▀▄ █▀▀▀ █▀▀█ █▀▀█ █▀▀█"))
        && screen.lines().any(|line| line.contains("ctrl+p commands"))
}

#[derive(Default)]
pub(super) struct State {
    pub(super) owner: Option<Owner>,
    pub(super) text: Option<String>,
    pub(super) revision: u64,
}

impl State {
    pub(super) fn observe(&mut self, owner: Option<Owner>) -> bool {
        if self.owner == owner {
            return false;
        }
        self.owner = owner;
        self.text = None;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub(super) fn commit(&mut self, text: &str) -> bool {
        let Some(text) = display_text(text) else { return false };
        self.revision = self.revision.wrapping_add(1);
        self.text = Some(text);
        true
    }

    pub(super) fn publish(&mut self, owner: &Owner, revision: u64, text: &str) -> bool {
        if self.owner.as_ref() != Some(owner) || self.revision != revision {
            return false;
        }
        let text = display_text(text);
        if self.text == text {
            return false;
        }
        self.text = text;
        self.revision = self.revision.wrapping_add(1);
        true
    }
}

fn display_text(text: &str) -> Option<String> {
    if text.chars().any(|ch| ch.is_control() && !ch.is_whitespace()) {
        return None;
    }
    let mut result = String::new();
    for word in text.split_whitespace() {
        if !result.is_empty() {
            if result.len() >= 1024 {
                result.push('…');
                return Some(result);
            }
            result.push(' ');
        }
        for ch in word.chars() {
            if result.len() + ch.len_utf8() > 1024 {
                result.push('…');
                return Some(result);
            }
            result.push(ch);
        }
    }
    (!result.is_empty()).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kimi_queue_footer_is_an_invalidation_signal_with_a_preceding_queue_row() {
        for suffix in [
            "ctrl-s to steer immediately",
            "will send after current task",
            "will send after compaction",
        ] {
            assert!(kimi_queue_present(&format!(
                "  ❯ pending\n  ↑ to edit · {suffix}\n│ > draft │"
            )));
        }
        assert!(!kimi_queue_present("↑ to edit · will send after current task\n│ > draft │"));
        assert!(!kimi_queue_present("❯ accepted\n● answer\n│ > draft │"));
    }

    #[test]
    fn opencode_home_requires_logo_and_composer_footer() {
        let logo = "█▀▀█ █▀▀█ █▀▀█ █▀▀▄ █▀▀▀ █▀▀█ █▀▀█ █▀▀█";
        assert!(opencode_home_present(&format!(
            "{logo}\n┃ draft\nshift+tab agents  ctrl+p commands"
        )));
        assert!(!opencode_home_present(&format!("{logo}\nordinary output")));
        assert!(!opencode_home_present("message body\nctrl+p commands"));
    }

    #[test]
    fn blank_and_control_input_do_not_replace_a_submission() {
        let mut state = State::default();
        state.observe(Some(Owner::Shell));
        assert!(state.commit("命令\r\n第二行\t🙂"));
        let revision = state.revision;
        for invalid in ["", " \r\n\t", "echo\u{1b}[0m", "\0"] {
            assert!(!state.commit(invalid));
        }
        assert_eq!(state.text.as_deref(), Some("命令 第二行 🙂"));
        assert_eq!(state.revision, revision);
        assert!(state.commit("命令\r\n第二行\t🙂"));
        assert_ne!(state.revision, revision, "repeated submissions remain distinct");
        assert!(display_text(&"🙂".repeat(1000)).unwrap().len() <= 1027);
    }

    #[test]
    fn owner_replacement_and_newer_submission_invalidate_old_reads() {
        let mut state = State::default();
        let owner = Owner::Agent { kind: "codex".into(), session: Some("a".into()), epoch: 1 };
        state.observe(Some(owner.clone()));
        let revision = state.revision;
        state.commit("new submission");
        assert!(!state.publish(&owner, revision, "late read"));
        for next in [
            Owner::Agent { kind: "codex".into(), session: Some("b".into()), epoch: 1 },
            Owner::Agent { kind: "pi".into(), session: Some("b".into()), epoch: 1 },
            Owner::Shell,
            owner.clone(),
        ] {
            state.observe(Some(next));
            assert!(state.text.is_none());
            assert!(!state.publish(&owner, revision, "old conversation"));
        }
    }
}
