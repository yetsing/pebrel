//! Project membership adapters. The pure model never owns a terminal or a tab.

use super::*;

pub(super) mod model;

impl NebulaWorkspace {
    pub(super) fn sync_pane_groups(&mut self) {
        self.pane_groups.sync(self.tabs.iter().flat_map(|tab| {
            match tab {
                WorkspaceTab::Terminal { panes, .. } => panes
                    .iter()
                    .filter_map(|pane| pane.project.clone().map(|project| (pane.id, project)))
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            }
        }));
    }

    pub(super) fn active_pane_project(&self) -> Option<model::PaneProject> {
        if self.settings_open {
            return None;
        }
        self.tabs.get(self.active)?.primary_terminal_pane()?.project.clone()
    }

    pub(super) fn observe_pane_project(&mut self, view: &Entity<TerminalView>, cx: &App) {
        let project = self.tabs.iter().find_map(|tab| match tab {
            WorkspaceTab::Terminal { panes, .. } => panes
                .iter()
                .find(|pane| pane.view.entity_id() == view.entity_id())
                .and_then(|pane| pane.project.as_ref()),
            _ => None,
        });
        if project
            .is_some_and(|project| project.observe_initial_directory(Path::new(&view.read(cx).cwd)))
            || self.pane_groups.needs_refresh()
        {
            self.sync_pane_groups();
        }
    }

    /// A pending lineage can resolve in a different window after a live transfer.
    /// The existing workspace watchdog calls this; no new task or I/O is needed.
    pub(super) fn refresh_pending_pane_groups(&mut self) -> bool {
        if self.pane_groups.needs_refresh() {
            self.sync_pane_groups();
            return true;
        }
        false
    }
}

#[cfg(all(test, feature = "gpui-test-support", windows))]
mod tests;
