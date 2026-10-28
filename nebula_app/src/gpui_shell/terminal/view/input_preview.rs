//! Only confirmed submissions feed the Windows pane sidebar.

use super::*;
use crate::ai_hook::{AiHookEvent, AiHookKind};
use std::time::{Duration, Instant};

mod state;
use state::{Owner, State};

#[cfg(all(test, windows, feature = "gpui-test-support"))]
mod tests;

#[derive(Default)]
pub(super) struct Preview {
    state: State,
    task: Option<gpui::Task<()>>,
    last_read: Option<Instant>,
    queued_without_order: bool,
    queue_clear_since: Option<Instant>,
    opencode_home_since: Option<Instant>,
}

impl TerminalView {
    #[cfg(all(test, feature = "gpui-test-support"))]
    pub(crate) fn install_preview_test_session(&mut self) -> std::sync::mpsc::Receiver<Msg> {
        let receiver = self.install_completion_test_session();
        self.suggest.suggest_env = crate::display::SuggestEnv::Local;
        receiver
    }

    fn preview_owner(&self) -> Option<Owner> {
        if !cfg!(windows) || self.is_remote_session() || self.exited.is_some() {
            return None;
        }
        Some(match self.runtime_chat_agent() {
            Some(agent) => Owner::Agent {
                kind: agent.kind,
                session: agent.session_id,
                epoch: self.ai_session_probe_epoch,
            },
            None => Owner::Shell,
        })
    }

    pub(crate) fn submitted_preview(&self) -> Option<&str> {
        (self.preview_owner() == self.input_preview.state.owner)
            .then(|| self.input_preview.state.text.as_deref())
            .flatten()
    }

    fn sync_preview_owner(&mut self, cx: &mut Context<Self>) {
        if self.input_preview.state.observe(self.preview_owner()) {
            self.input_preview.task = None;
            self.input_preview.last_read = None;
            self.input_preview.queued_without_order = false;
            self.input_preview.queue_clear_since = None;
            self.input_preview.opencode_home_since = None;
            cx.emit(TerminalViewEvent::PreviewChanged);
        }
    }

    pub(super) fn preview_shell_report(&mut self, name: &str, value: &str, cx: &mut Context<Self>) {
        if name != crate::completion_context::INPUT_VAR
            || self.preview_owner() != Some(Owner::Shell)
        {
            return;
        }
        let Some((owner, text)) = value.split_once('\n') else { return };
        if text.len() > 48 * 1024 || !self.suggest.completion_context.owns_current(owner) {
            return;
        }
        self.sync_preview_owner(cx);
        if self.input_preview.state.commit(text) {
            cx.emit(TerminalViewEvent::PreviewChanged);
        }
    }

    pub(super) fn preview_hook(&mut self, event: &AiHookEvent, cx: &mut Context<Self>) {
        self.sync_preview_owner(cx);
        if (event.kind != AiHookKind::UserMessage
            && event.user_message.is_none()
            && !event.user_message_reset)
            || event.pane != Some(self.pane_id)
        {
            return;
        }
        let Some(Owner::Agent { kind, session: Some(session), .. }) =
            self.input_preview.state.owner.as_ref()
        else {
            return;
        };
        if kind != &event.source || Some(session.as_str()) != event.session_id.as_deref() {
            return;
        }
        if event.source == "kimi" && !event.user_message_queued {
            // Kimi hooks omit the originating agent ID. Read only the exact
            // session's main wire instead of accepting a subagent's hook text.
            self.refresh_input_preview(cx);
            return;
        }
        if event.user_message_reset
            || event.kind == AiHookKind::UserMessage && event.user_message.is_none()
        {
            self.input_preview.state.text = None;
            self.input_preview.state.revision = self.input_preview.state.revision.wrapping_add(1);
            cx.emit(TerminalViewEvent::PreviewChanged);
        }
        if event.user_message_queued {
            // These hooks have no source sequence or root-agent identity.
            // Processing an older queued item must not restore stale text.
            self.input_preview.queued_without_order = true;
            self.input_preview.queue_clear_since = None;
            self.input_preview.task = None;
            self.input_preview.last_read = None;
            self.input_preview.state.text = None;
            self.input_preview.state.revision = self.input_preview.state.revision.wrapping_add(1);
            cx.emit(TerminalViewEvent::PreviewChanged);
        } else if let Some(text) = &event.user_message
            && self.input_preview.state.commit(text)
        {
            cx.emit(TerminalViewEvent::PreviewChanged);
        }
    }

    pub(super) fn refresh_input_preview(&mut self, cx: &mut Context<Self>) {
        self.sync_preview_owner(cx);
        self.observe_kimi_queue(cx);
        self.observe_opencode_home(cx);
        if self.input_preview.queued_without_order
            || self.input_preview.task.is_some()
            || self.input_preview.last_read.is_some_and(|at| at.elapsed() < Duration::from_secs(2))
        {
            return;
        }
        let Some(owner @ Owner::Agent { .. }) = self.preview_owner() else { return };
        let work = match &owner {
            Owner::Agent { kind, .. } if kind == "codex" => {
                let Ok(identity) = self.runtime_conversation_identity() else { return };
                let Ok(source) = self.runtime_conversation_source(&identity) else { return };
                cx.background_executor().spawn(async move {
                    crate::assistant_answer::conversation::latest_user(source)
                })
            },
            Owner::Agent { kind, session: Some(session), .. } if kind == "kimi" => {
                if self.exec_context.is_none() {
                    return;
                }
                let Some(home) =
                    std::env::var_os("KIMI_CODE_HOME").map(std::path::PathBuf::from).or_else(
                        || crate::platform::dirs::home_dir().map(|home| home.join(".kimi-code")),
                    )
                else {
                    return;
                };
                let session = session.clone();
                cx.background_executor().spawn(async move {
                    crate::assistant_answer::conversation::latest_kimi_user(&home, &session)
                })
            },
            _ => return,
        };
        let revision = self.input_preview.state.revision;
        let pane = self.pane_id;
        self.input_preview.last_read = Some(Instant::now());
        self.input_preview.task = Some(cx.spawn(async move |this, cx| {
            let text = work.await;
            let _ = this.update(cx, |view, cx| {
                view.input_preview.task = None;
                if view.pane_id == pane
                    && view.preview_owner().as_ref() == Some(&owner)
                    && let Some(text) = text
                    && view.input_preview.state.publish(&owner, revision, &text)
                {
                    cx.emit(TerminalViewEvent::PreviewChanged);
                    cx.notify();
                }
            });
        }));
    }

    fn observe_kimi_queue(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.input_preview.state.owner.as_ref(), Some(Owner::Agent {kind, ..}) if kind == "kimi")
        {
            return;
        }
        let Some((screen, _)) = self.runtime_screen_state() else { return };
        if state::kimi_queue_present(&screen) {
            self.input_preview.queue_clear_since = None;
            if !self.input_preview.queued_without_order {
                self.input_preview.queued_without_order = true;
                self.input_preview.task = None;
                self.input_preview.state.text = None;
                self.input_preview.state.revision =
                    self.input_preview.state.revision.wrapping_add(1);
                cx.emit(TerminalViewEvent::PreviewChanged);
            }
        } else if self.input_preview.queued_without_order {
            // A missing footer alone does not prove the queue drained. Require
            // native completion and a stable visible composer before rereading
            // the main wire; this observation never supplies preview content.
            if matches!(self.runtime_task_state(), crate::runtime_api::RuntimeTaskState::Finished)
                && screen.lines().any(|line| line.trim_start().starts_with("│ >"))
            {
                let since = self.input_preview.queue_clear_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_secs(2) {
                    self.input_preview.queued_without_order = false;
                    self.input_preview.queue_clear_since = None;
                    self.input_preview.last_read = None;
                }
            } else {
                self.input_preview.queue_clear_since = None;
            }
        }
    }

    fn observe_opencode_home(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.input_preview.state.owner.as_ref(), Some(Owner::Agent {kind, ..}) if kind == "opencode")
        {
            return;
        }
        let home = self
            .runtime_screen_state()
            .is_some_and(|(screen, _)| state::opencode_home_present(&screen));
        if !home || self.runtime_task_state() != crate::runtime_api::RuntimeTaskState::Idle {
            self.input_preview.opencode_home_since = None;
            return;
        }
        let since = self.input_preview.opencode_home_since.get_or_insert_with(Instant::now);
        // The empty page has no native session ID until the first submission.
        // Allow a repaint after admission; a stable home can only clear text.
        if since.elapsed() >= Duration::from_secs(2) && self.input_preview.state.text.is_some() {
            self.input_preview.state.text = None;
            self.input_preview.state.revision = self.input_preview.state.revision.wrapping_add(1);
            cx.emit(TerminalViewEvent::PreviewChanged);
        }
    }
}
