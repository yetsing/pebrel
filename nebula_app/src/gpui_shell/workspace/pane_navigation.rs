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

    pub(super) fn focus_active(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings_open {
            if let Some((view, _)) = self.settings_surface.as_ref() {
                let focus = view.read(cx).focus_handle(cx);
                window.defer(cx, move |window, cx| window.focus(&focus, cx));
            }
            return;
        }
        let focus = match self.tabs.get(self.active) {
            Some(tab @ WorkspaceTab::Terminal { .. }) => match tab.focused_view() {
                Some(view) => view.read(cx).focus_handle(cx),
                None => return,
            },
            Some(WorkspaceTab::Settings { view, .. }) => view.read(cx).focus_handle(cx),
            Some(WorkspaceTab::Document { view, .. }) => view.read(cx).focus_handle(cx),
            Some(WorkspaceTab::Code { view, .. }) => view.read(cx).focus_handle(cx),
            Some(WorkspaceTab::Image { .. }) | None => return,
        };
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
    }
}
