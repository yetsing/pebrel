//! Opt-in observation for an isolated debug product, never a production API.

use super::*;

impl NebulaWorkspace {
    pub(in crate::gpui_shell::workspace) fn capture_sidebar_qa(&self, cx: &App) {
        let Some(root) = std::env::var_os("PEBREL_SIDEBAR_QA_DIR").map(std::path::PathBuf::from)
        else {
            return;
        };
        if std::env::var_os("PEBREL_CONFIG_DIR").map(std::path::PathBuf::from)
            != Some(root.join("config"))
        {
            return;
        }
        let panes = self
            .tabs
            .iter()
            .filter_map(|tab| {
                if let WorkspaceTab::Terminal { panes, .. } = tab { Some(panes) } else { None }
            })
            .flatten()
            .map(|pane| {
                let view = pane.view.read(cx);
                serde_json::json!({"pane_id":pane.id,"agent":view.runtime_agent(),
                "preview":view.submitted_preview(),"state":view.runtime_task_state()})
            })
            .collect::<Vec<_>>();
        let snapshot = serde_json::to_vec(&serde_json::json!({
            "pid":std::process::id(),"window_id":self.runtime_window_id,"panes":panes,
        }))
        .unwrap();
        // The fixture owns this directory. No transcript or user settings are copied.
        cx.background_executor()
            .spawn(async move {
                let target = root.join("live-preview.json");
                let pending = root.join("live-preview.pending");
                if std::fs::write(&pending, snapshot).is_ok() {
                    let _ = std::fs::rename(pending, target);
                }
            })
            .detach();
    }
}
