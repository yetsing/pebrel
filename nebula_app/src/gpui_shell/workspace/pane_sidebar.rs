//! Project headers and live pane navigation for the Windows sidebar.

use super::*;
use crate::platform::project_directory::ProjectDirectory;

mod actions;
mod branches;
mod colors;
mod project_header;
#[cfg(all(feature = "gpui-test-support", debug_assertions))]
mod qa;
mod view;

pub(super) struct SidebarState {
    scroll: gpui::ScrollHandle,
    picker: Option<gpui::Task<()>>,
    branches: branches::Branches,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self { scroll: gpui::ScrollHandle::new(), picker: None, branches: Default::default() }
    }
}

impl SidebarState {
    pub(super) fn sync_projects<'a>(
        &mut self,
        directories: impl Iterator<Item = &'a ProjectDirectory>,
    ) {
        self.branches.sync(directories);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Row {
    Project(ProjectDirectory),
    Pane(u64),
    Tab(crate::runtime_api::tabs::TabId),
}

impl NebulaWorkspace {
    pub(super) fn sidebar_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for group in self.pane_groups.ordered_groups() {
            rows.push(Row::Project(group.directory.clone()));
            for &id in &group.panes {
                seen.insert(id);
                if !group.collapsed {
                    rows.push(Row::Pane(id));
                }
            }
        }
        for id in self.pane_groups.pending_panes() {
            rows.push(Row::Pane(id));
            seen.insert(id);
        }
        // Unsupported transports stay reachable without inventing local projects.
        for tab in &self.tabs {
            if let WorkspaceTab::Terminal { panes, .. } = tab {
                for pane in panes {
                    if seen.insert(pane.id) {
                        rows.push(Row::Pane(pane.id));
                    }
                }
            }
        }
        for (index, tab) in self.tabs.iter().enumerate() {
            if !tab.is_terminal() {
                rows.push(Row::Tab(self.tab_meta[index].runtime_id.clone()));
            }
        }
        rows
    }

    pub(super) fn reveal_sidebar_selection(&mut self) {
        if !cfg!(windows) || self.settings_open {
            return;
        }
        let selected = self
            .active_terminal_pane_id()
            .map(|id| {
                self.pane_groups.reveal(id);
                Row::Pane(id)
            })
            .or_else(|| {
                self.tab_meta.get(self.active).map(|meta| Row::Tab(meta.runtime_id.clone()))
            });
        if let Some(index) =
            self.sidebar_rows().iter().position(|row| Some(row) == selected.as_ref())
        {
            self.pane_sidebar.scroll.scroll_to_item(index);
        }
    }
}

#[cfg(all(test, feature = "gpui-test-support", windows))]
mod tests;

#[cfg(all(test, feature = "gpui-test-support", windows))]
mod native_tests;
