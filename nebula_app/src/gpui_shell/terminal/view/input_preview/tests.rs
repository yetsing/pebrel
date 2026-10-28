use super::*;
use crate::ai_agents::AgentStatus;
use gpui::TestAppContext;
use serde_json::{Value, json};

fn event(source: &str, payload: Value) -> AiHookEvent {
    let mut event = crate::ai_hook::parse_remote_envelope(
        format!("nebula-hook/1 source={source} pane=42\n{payload}").as_bytes(),
        Some(42),
    )
    .unwrap();
    event.agent_pid = Some(1042);
    event
}

fn shell(view: &mut TerminalView, cx: &mut Context<TerminalView>) {
    view.suggest.suggest_env = crate::display::SuggestEnv::Local;
    view.process_event(
        TermEvent::UserVar { name: "pebrel_shell".into(), value: "owner".into() },
        cx,
    );
}

#[gpui::test]
fn only_accepted_local_shell_input_updates_preview(cx: &mut TestAppContext) {
    let (view, window, _) = super::super::startup_tests::open(cx);
    view.update(window, |view, cx| {
        shell(view, cx);
        let report =
            |text: &str| TermEvent::UserVar { name: "pebrel_input".into(), value: text.into() };
        view.process_event(report("owner\nWrite-Output 第一行\nWrite-Output 第二行"), cx);
        assert_eq!(view.submitted_preview(), Some("Write-Output 第一行 Write-Output 第二行"));
        let revision = view.input_preview.state.revision;
        view.suggest.last_committed = "unconfirmed Runtime input".into();
        view.process_event(report("foreign\nwrong pane"), cx);
        view.process_event(report("owner\n \n"), cx);
        view.process_event(report("owner\n\u{1b}[A"), cx);
        assert_eq!(view.input_preview.state.revision, revision);
        view.process_event(report("owner\nWrite-Output 第一行\nWrite-Output 第二行"), cx);
        assert_ne!(view.input_preview.state.revision, revision);
        view.running_program = Some("codex".into());
        view.refresh_input_preview(cx);
        assert!(view.submitted_preview().is_none(), "a launch command is not an Agent message");
        view.process_event(report("owner\nlate shell input"), cx);
        assert!(view.submitted_preview().is_none());
        view.clear_foreground_agent_state(cx);
        assert!(view.submitted_preview().is_none(), "returning to shell must not restore old text");
        view.process_event(report("owner\necho fresh"), cx);
        assert_eq!(view.submitted_preview(), Some("echo fresh"));
        view.suggest.suggest_env = crate::display::SuggestEnv::Wsl { distro: "Debian".into() };
        assert!(view.submitted_preview().is_none());
    });
}

#[gpui::test]
fn user_messages_are_scoped_and_do_not_change_turn_activity(cx: &mut TestAppContext) {
    let (view, window, _) = super::super::startup_tests::open(cx);
    view.update(window, |view, cx| {
        shell(view, cx);
        let first = "preview-opencode-first";
        assert!(view.handle_ai_hook(&event("opencode", json!({"kind":"session-start","session_id":first,"bridge_sequence":1})), cx));
        assert_eq!(view.agent_activity.status(), AgentStatus::Idle);
        assert!(view.handle_ai_hook(&event("opencode", json!({"kind":"user-message","session_id":first,"bridge_sequence":2,"user_message":"用户\n原文"})), cx));
        assert_eq!(view.submitted_preview(), Some("用户 原文"));
        assert_eq!(view.agent_activity.status(), AgentStatus::Idle, "preview does not start a turn");
        assert!(view.handle_ai_hook(&event("opencode", json!({"kind":"prompt","session_id":first,"bridge_sequence":3,"message":"not user text"})), cx));
        assert_eq!(view.submitted_preview(), Some("用户 原文"));
        assert!(view.handle_ai_hook(&event("opencode", json!({"kind":"session-start","session_id":"preview-opencode-second","bridge_sequence":4})), cx));
        assert!(view.submitted_preview().is_none());
        view.handle_ai_hook(&event("opencode", json!({"kind":"user-message","session_id":first,"bridge_sequence":5,"user_message":"old conversation"})), cx);
        assert!(view.submitted_preview().is_none());
    });
}

#[gpui::test]
fn pi_branch_projection_clears_old_text_and_late_sequences_cannot_restore_it(
    cx: &mut TestAppContext,
) {
    let (view, window, _) = super::super::startup_tests::open(cx);
    view.update(window, |view, cx| {
        shell(view, cx);
        let payload = |kind: &str, seq: u64, text: Value, reset: bool| json!({
            "kind":kind, "session_id":"preview-pi-branch", "bridge_instance":"preview-pi-instance",
            "bridge_sequence":seq, "user_message":text, "preview_reset":reset,
        });
        view.handle_ai_hook(&event("pi", payload("session-start", 1, json!(null), false)), cx);
        view.handle_ai_hook(&event("pi", payload("prompt", 2, json!("branch A"), false)), cx);
        assert_eq!(view.submitted_preview(), Some("branch A"));
        view.handle_ai_hook(&event("pi", payload("session-start", 4, json!("branch B"), true)), cx);
        assert_eq!(view.submitted_preview(), Some("branch B"));
        view.handle_ai_hook(&event("pi", payload("prompt", 3, json!("late branch A"), false)), cx);
        assert_eq!(view.submitted_preview(), Some("branch B"));
        view.handle_ai_hook(&event("pi", payload("session-start", 5, json!(null), true)), cx);
        assert!(view.submitted_preview().is_none());
    });
}

#[gpui::test]
fn kimi_hook_text_cannot_replace_native_root_preview_and_queues_fail_closed(
    cx: &mut TestAppContext,
) {
    let (view, window, _) = super::super::startup_tests::open(cx);
    view.update(window, |view, cx| {
        shell(view, cx);
        let payload = |name: &str, turn: u64, text: &str| {
            json!({
                "hook_event_name":name,"session_id":"preview-kimi-turns","turn_id":turn,
                "origin_kind":"user", "agent_id":"main", "prompt":[{"type":"text","text":text}],
            })
        };
        view.handle_ai_hook(&event("kimi", payload("SessionStart", 0, "")), cx);
        view.input_preview.state.commit("native root message");
        view.handle_ai_hook(&event("kimi", payload("TurnStarted", 2, "newer")), cx);
        assert_eq!(view.submitted_preview(), Some("native root message"));
        view.handle_ai_hook(&event("kimi", payload("TurnStarted", 1, "older")), cx);
        assert_eq!(view.submitted_preview(), Some("native root message"));
        let mut system = payload("TurnStarted", 3, "system trigger");
        system["origin_kind"] = json!("system_trigger");
        assert!(
            crate::ai_hook::parse_remote_envelope(
                format!("nebula-hook/1 source=kimi\n{system}").as_bytes(),
                Some(42)
            )
            .is_none()
        );
        let mut queued = payload("UserPromptQueued", 3, "queued");
        queued["prompt_id"] = json!("queued-message");
        queued["queue_length"] = json!(1);
        view.handle_ai_hook(&event("kimi", queued), cx);
        assert!(view.submitted_preview().is_none());
        view.handle_ai_hook(
            &event("kimi", payload("TurnStarted", 4, "cannot establish submission order")),
            cx,
        );
        assert!(view.submitted_preview().is_none());
    });
}

#[gpui::test]
fn kimi_tui_queue_invalidates_only_and_native_completion_allows_a_fresh_read(
    cx: &mut TestAppContext,
) {
    let (view, window, _) = super::super::startup_tests::open(cx);
    view.update(window, |view, cx| {
        shell(view, cx);
        view.handle_ai_hook(&event("kimi", json!({"hook_event_name":"SessionStart","session_id":"preview-kimi-tui"})), cx);
        view.input_preview.state.commit("native accepted text");
        let paint = |view: &mut TerminalView, text: &str| {
            super::super::startup_tests::feed(view, format!("\x1b[2J\x1b[H{text}").as_bytes());
        };
        paint(view, "  ❯ queued text that must never become a preview\r\n  ↑ to edit · ctrl-s to steer immediately\r\n│ > draft │");
        view.refresh_input_preview(cx);
        assert!(view.submitted_preview().is_none());
        assert!(view.input_preview.queued_without_order);
        paint(view, "│ > a new draft │");
        view.handle_ai_hook(&event("kimi", json!({"hook_event_name":"UserPromptSubmit","session_id":"preview-kimi-tui"})), cx);
        view.input_preview.queue_clear_since = Some(Instant::now() - Duration::from_secs(3));
        view.refresh_input_preview(cx);
        assert!(view.input_preview.queued_without_order, "a missing footer cannot finish a running turn");
        view.handle_ai_hook(&event("kimi", json!({"hook_event_name":"Stop","session_id":"preview-kimi-tui"})), cx);
        view.refresh_input_preview(cx);
        view.input_preview.queue_clear_since = Some(Instant::now() - Duration::from_secs(3));
        view.refresh_input_preview(cx);
        assert!(!view.input_preview.queued_without_order);
        assert!(view.submitted_preview().is_none(), "only the native reader may restore text");
    });
}

#[gpui::test]
fn opencode_new_empty_conversation_cannot_keep_the_previous_preview(cx: &mut TestAppContext) {
    let (view, window, _) = super::super::startup_tests::open(cx);
    view.update(window, |view, cx| {
        shell(view, cx);
        view.handle_ai_hook(&event("opencode", json!({"kind":"session-start","session_id":"preview-oc-home-old","bridge_sequence":1})), cx);
        view.handle_ai_hook(&event("opencode", json!({"kind":"user-message","session_id":"preview-oc-home-old","bridge_sequence":2,"user_message":"old conversation"})), cx);
        super::super::startup_tests::feed(view, "\x1b[2J\x1b[H█▀▀█ █▀▀█ █▀▀█ █▀▀▄ █▀▀▀ █▀▀█ █▀▀█ █▀▀█\r\nctrl+p commands".as_bytes());
        view.refresh_input_preview(cx);
        assert_eq!(view.submitted_preview(), Some("old conversation"), "one repaint is not a new conversation");
        view.input_preview.opencode_home_since = Some(Instant::now() - Duration::from_secs(3));
        view.refresh_input_preview(cx);
        assert!(view.submitted_preview().is_none());
        super::super::startup_tests::feed(view, b"\x1b[2J\x1b[Hnew conversation body");
        view.handle_ai_hook(&event("opencode", json!({"kind":"session-start","session_id":"preview-oc-home-new","bridge_sequence":3})), cx);
        view.handle_ai_hook(&event("opencode", json!({"kind":"user-message","session_id":"preview-oc-home-new","bridge_sequence":4,"user_message":"fresh accepted input"})), cx);
        assert_eq!(view.submitted_preview(), Some("fresh accepted input"));
    });
}

#[gpui::test]
fn opencode_verified_bridge_can_switch_without_a_resolved_agent_pid(cx: &mut TestAppContext) {
    let (view, window, _) = super::super::startup_tests::open(cx);
    view.update(window, |view, cx| {
        shell(view, cx);
        let input = |kind: &str, session: &str, sequence: u64, instance: &str| {
            let mut event = event("opencode", json!({"kind":kind,"session_id":session,"bridge_instance":instance,"bridge_sequence":sequence,"user_message":"accepted"}));
            event.agent_pid = None;
            event
        };
        view.handle_ai_hook(&input("session-start", "preview-oc-no-pid-old", 1, "verified-process"), cx);
        view.handle_ai_hook(&input("user-message", "preview-oc-no-pid-old", 2, "verified-process"), cx);
        assert_eq!(view.submitted_preview(), Some("accepted"));
        view.handle_ai_hook(&input("session-start", "preview-oc-no-pid-new", 3, "foreign-process"), cx);
        assert_eq!(view.runtime_chat_agent().unwrap().session_id.as_deref(), Some("preview-oc-no-pid-old"));
        view.handle_ai_hook(&input("session-start", "preview-oc-no-pid-new", 3, "verified-process"), cx);
        assert!(view.submitted_preview().is_none());
        view.handle_ai_hook(&input("user-message", "preview-oc-no-pid-new", 4, "verified-process"), cx);
        view.handle_ai_hook(&input("session-start", "preview-oc-no-pid-old", 2, "verified-process"), cx);
        assert_eq!(view.runtime_chat_agent().unwrap().session_id.as_deref(), Some("preview-oc-no-pid-new"));
        assert_eq!(view.submitted_preview(), Some("accepted"));
    });
}
