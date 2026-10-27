//! Local terminal creation and frozen project context.

use super::*;

impl NebulaWorkspace {
    pub(super) fn add_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 旧壳合同（window_context `spawn_tab` 一族）：新 tab 的 cwd 先取
        // 设置页的「启动目录」（存在且是目录才算数），否则继承聚焦 pane
        // 的 cwd——`startup_directory=` 因此在两壳有同一效果。WSL 来宾目录
        // 见 `tab_duplication::CopyKind::NewTab`。
        let (launch, cwd) = match Self::startup_directory() {
            Some(dir) => (shell_launch::configured_local_launch(cx), Some(dir)),
            None => self.new_tab_from_focused(cx),
        };
        let project = self.active_pane_project();
        self.add_terminal_for_project(launch, cwd, None, project, window, cx);
    }

    /// 设置页「启动目录」：宿主验证本机路径，WSL 验证来宾路径。
    pub(super) fn startup_directory() -> Option<std::path::PathBuf> {
        shell_launch::configured_directory()
    }

    /// 创建一个 pane 实体（分配 id、spawn 会话、挂宿主订阅）；调用方决定
    /// 它进新 tab 还是接进某棵分屏树。
    pub(super) fn new_pane(
        &mut self,
        grid: (u16, u16),
        launch: crate::gpui_shell::terminal::view::TerminalLaunch,
        command: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> TerminalPane {
        self.new_pane_for_project(grid, launch, command, None, window, cx)
    }

    pub(super) fn new_pane_for_project(
        &mut self,
        grid: (u16, u16),
        launch: crate::gpui_shell::terminal::view::TerminalLaunch,
        command: Option<String>,
        project: Option<pane_groups::model::PaneProject>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> TerminalPane {
        let pane_id = self.next_pane_id;
        self.next_pane_id = self.next_pane_id.saturating_add(1);
        let view = cx.new(|cx| TerminalView::new(pane_id, grid, launch, window, cx));
        let subscription = cx.subscribe_in(&view, window, Self::on_terminal_event);
        let project = (cfg!(windows) && !view.read(cx).is_remote_session()).then(|| {
            let project = project
                .or_else(|| {
                    crate::platform::project_directory::ProjectDirectory::new(Path::new(
                        &view.read(cx).cwd,
                    ))
                    .map(pane_groups::model::PaneProject::fixed)
                })
                .unwrap_or_default();
            project.observe_initial_directory(Path::new(&view.read(cx).cwd));
            project
        });
        if let Some(command) = command {
            view.update(cx, |view, cx| view.run_command(command, cx));
        }
        TerminalPane {
            id: pane_id,
            project,
            custom_name: None,
            name_history: Vec::new(),
            view,
            _subscription: subscription,
        }
    }

    /// 现网格（聚焦终端）或开窗反推的目标网格：让新 pane 的 PTY 出生即
    /// 接近最终几何，启动接近零 resize。
    pub(super) fn inherited_grid(&self, cx: &App) -> (u16, u16) {
        self.tabs
            .iter()
            .find_map(|tab| {
                tab.focused_view().map(|view| {
                    let view = view.read(cx);
                    (view.grid_cols() as u16, view.grid_rows() as u16)
                })
            })
            .unwrap_or(self.initial_grid)
    }

    pub(super) fn add_terminal_at(
        &mut self,
        cwd: Option<std::path::PathBuf>,
        command: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> u64 {
        // 默认 shell 只在“创建新 Tab”的这一刻取样，并把实际 program/args
        // 一起冻结进 Tab launch。设置页随后改默认值只影响下一次创建；冷
        // 恢复也按本 Tab 的 launch 重建，不会把混合工作区抹成同一种 shell。
        let launch_session = shell_launch::configured_at(cx, cwd.as_deref());
        self.add_terminal_with(launch_session, cwd, command, window, cx)
    }

    /// 用一份指定的 launch 身份建 Tab。默认路径（`add_terminal_at`）冻结
    /// 设置里的默认 shell；Shell 选择弹窗则把用户挑中的那台传进来。
    pub(super) fn add_terminal_with(
        &mut self,
        launch_session: crate::session::LaunchSession,
        cwd: Option<std::path::PathBuf>,
        command: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> u64 {
        self.add_terminal_for_project(launch_session, cwd, command, None, window, cx)
    }

    pub(super) fn add_terminal_for_project(
        &mut self,
        launch_session: crate::session::LaunchSession,
        cwd: Option<std::path::PathBuf>,
        command: Option<String>,
        project: Option<pane_groups::model::PaneProject>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> u64 {
        if self.settings_open {
            self.leave_settings(window, cx);
        }
        let shell_tag = Self::launch_shell_tag(&launch_session);
        let grid = self.inherited_grid(cx);
        let launch = Self::terminal_launch_from_session(&launch_session, cwd);
        let pane = self.new_pane_for_project(grid, launch, command, project, window, cx);
        let focused = pane.id;
        let tab = WorkspaceTab::Terminal {
            tree: SplitTree::leaf(pane.id),
            panes: vec![pane],
            focused,
            zoomed: false,
            broadcast: false,
        };
        let position = nebula_settings::RuntimeSettings::load().new_tab_position;
        let at = new_tab_insert_index(position, self.active, self.tabs.len());
        self.insert_tab_at(
            at,
            tab,
            TabMeta { shell_tag, launch: Some(launch_session), ..TabMeta::default() },
        );
        self.active = at;
        self.reveal_active_tab();
        self.focus_active(window, cx);
        self.sync_side_panel_to_active(true, cx);
        cx.notify();
        focused
    }
}
