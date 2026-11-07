//! Pane selection and deferred view focus.

use super::*;

impl NebulaWorkspace {
    pub(super) fn activate_sidebar_pane(
        &mut self,
        id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.tab_of_pane(id) else { return };
        self.activate_tab(index, window, cx);
        self.clear_reader_focus(cx);
        let unzoom = matches!(&self.tabs[index], WorkspaceTab::Terminal { focused, zoomed: true, .. } if *focused != id);
        if unzoom {
            if let WorkspaceTab::Terminal { zoomed, .. } = &mut self.tabs[index] {
                *zoomed = false;
            }
            self.mark_structural_resize(index, cx);
        }
        self.focus_pane(index, id, window, cx);
        self.focus_active(window, cx);
        self.reveal_sidebar_selection();
        cx.notify();
    }
}
