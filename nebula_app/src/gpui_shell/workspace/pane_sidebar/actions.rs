use super::super::pane_groups::model::PaneProject;
use super::*;
use crate::i18n::Message;
use crate::session::LaunchSession;

pub(super) fn supports_project(launch: &LaunchSession) -> bool {
    match launch {
        LaunchSession::Default => true,
        LaunchSession::Shell { program, .. } => !crate::shell_detect::is_wsl_launcher(program),
        LaunchSession::Profile { command, .. } => !crate::shell_detect::is_wsl_launcher(command),
        _ => false,
    }
}

impl NebulaWorkspace {
    pub(super) fn toggle_sidebar_project(
        &mut self,
        directory: &ProjectDirectory,
        cx: &mut Context<Self>,
    ) {
        self.pane_groups.toggle_collapsed(directory);
        cx.notify();
    }

    pub(super) fn pin_sidebar_project(
        &mut self,
        directory: &ProjectDirectory,
        cx: &mut Context<Self>,
    ) {
        self.pane_groups.toggle_pinned(directory);
        if let Some(index) =
            self.sidebar_rows().iter().position(|row| row == &Row::Project(directory.clone()))
        {
            self.pane_sidebar.scroll.scroll_to_item(index);
        }
        cx.notify();
    }

    pub(in crate::gpui_shell::workspace) fn begin_sidebar_tab_drag(
        &mut self,
        id: u64,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(source) = self.tab_of_pane(id) else { return };
        self.tab_drag = Some(super::super::tab_drag::TabDrag {
            source,
            cross_window: self.cross_window_drag_payload(source, cx),
            cross_window_target: None,
            press_x: f32::from(event.position.x),
            press_y: f32::from(event.position.y),
            axis: super::super::tab_drag::TabDragAxis::PaneSidebar { target: None },
            pitch: 1.0,
            offset: 0.0,
            active: false,
            force_tear_out: event.modifiers.alt,
            dock: None,
        });
        cx.notify();
    }

    pub(in crate::gpui_shell::workspace) fn sidebar_drop_target(
        &self,
        position: gpui::Point<gpui::Pixels>,
    ) -> Option<u64> {
        if self.sidebar_collapsed
            || self.tabs_section_collapsed
            || !self.pane_sidebar.scroll.bounds().contains(&position)
        {
            return None;
        }
        let unscrolled = position - self.pane_sidebar.scroll.offset();
        self.sidebar_rows().iter().enumerate().find_map(|(index, row)| {
            let Row::Pane(id) = row else { return None };
            self.pane_sidebar
                .scroll
                .bounds_for_item(index)
                .is_some_and(|bounds| bounds.contains(&unscrolled))
                .then_some(*id)
        })
    }

    pub(super) fn activate_sidebar_document(
        &mut self,
        id: &crate::runtime_api::tabs::TabId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.tab_meta.iter().position(|meta| &meta.runtime_id == id) {
            self.activate_tab(index, window, cx);
            self.focus_active(window, cx);
        }
    }

    pub(in crate::gpui_shell::workspace) fn add_project_session(
        &mut self,
        directory: ProjectDirectory,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let launch = shell_launch::configured_at(cx, Some(directory.path()));
        if !supports_project(&launch) {
            return;
        }
        self.add_terminal_for_project(
            launch,
            Some(directory.path().to_owned()),
            None,
            Some(PaneProject::fixed(directory)),
            window,
            cx,
        );
    }

    pub(in crate::gpui_shell::workspace) fn open_sidebar_project(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pane_sidebar.picker.is_some()
            || !supports_project(&shell_launch::configured_local_launch(cx))
        {
            return;
        }
        let language = crate::gpui_shell::config::ui_language(cx);
        let picked = crate::platform::file_picker::directory(
            window,
            cx,
            language.text(Message::WorkspaceSidebarOpenProject),
        );
        self.pane_sidebar.picker = Some(cx.spawn_in(window, async move |this, cx| {
            let result = match picked.await {
                None => None,
                Some(path) => Some(
                    cx.background_executor()
                        .spawn(async move {
                            let directory =
                                path.is_dir().then(|| ProjectDirectory::new(&path)).flatten();
                            (path, directory)
                        })
                        .await,
                ),
            };
            let _ = this.update_in(cx, |workspace, window, cx| {
                workspace.pane_sidebar.picker = None;
                if let Some((path, directory)) = result {
                    if let Some(directory) = directory {
                        workspace.add_project_session(directory, window, cx);
                    } else {
                        crate::gpui_shell::toast::banner(
                            window,
                            cx,
                            crate::display::ToastKind::Warning,
                            language.format(
                                Message::WorkspaceSidebarDirectoryUnavailable,
                                &[("path", &path.to_string_lossy())],
                            ),
                        );
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(in crate::gpui_shell::workspace) fn close_sidebar_pane(
        &mut self,
        id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.tab_of_pane(id) {
            self.request_close_pane(index, id, window, cx);
        }
    }

    pub(in crate::gpui_shell::workspace) fn drop_sidebar_tab(
        &mut self,
        payload: &windowing::CrossWindowTabDrag,
        target_pane: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if payload.source_window_id() == self.runtime_window_id {
            if let (Some(source), Some(target)) =
                (self.tab_of_pane(payload.source_pane_id()), self.tab_of_pane(target_pane))
            {
                self.move_tab(source, target, window, cx);
            }
        } else {
            self.accept_cross_window_tab(payload, None, window, cx);
        }
    }
}
