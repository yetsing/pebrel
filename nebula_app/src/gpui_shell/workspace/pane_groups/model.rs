//! Window-local projection of live pane ownership; no terminal or GPUI types.

use std::cell::OnceCell;
use std::path::Path;
use std::rc::Rc;

use crate::platform::project_directory::ProjectDirectory;

/// Splits created before the first cwd report share the same one-shot binding.
/// The binding travels with a live pane, including across windows.
#[derive(Clone, Debug, Default)]
pub(in crate::gpui_shell::workspace) struct PaneProject(Rc<OnceCell<ProjectDirectory>>);

impl PaneProject {
    pub(in crate::gpui_shell::workspace) fn directory(&self) -> Option<&ProjectDirectory> {
        self.0.get()
    }

    pub(in crate::gpui_shell::workspace) fn fixed(directory: ProjectDirectory) -> Self {
        Self(Rc::new(OnceCell::from(directory)))
    }

    pub(in crate::gpui_shell::workspace) fn observe_initial_directory(&self, path: &Path) -> bool {
        if self.directory().is_some() {
            return false;
        }
        ProjectDirectory::new(path).is_some_and(|directory| self.0.set(directory).is_ok())
    }
}

#[derive(Debug)]
pub(in crate::gpui_shell::workspace) struct ProjectGroup {
    pub(in crate::gpui_shell::workspace) directory: ProjectDirectory,
    pub(in crate::gpui_shell::workspace) panes: Vec<u64>,
    pub(in crate::gpui_shell::workspace) collapsed: bool,
    pub(in crate::gpui_shell::workspace) pinned: bool,
}

#[derive(Debug)]
struct PaneEntry {
    id: u64,
    project: PaneProject,
    grouped: bool,
}

#[derive(Debug, Default)]
pub(in crate::gpui_shell::workspace) struct PaneGroups {
    groups: Vec<ProjectGroup>,
    panes: Vec<PaneEntry>,
}

impl PaneGroups {
    pub(in crate::gpui_shell::workspace) fn groups(&self) -> &[ProjectGroup] {
        &self.groups
    }

    pub(in crate::gpui_shell::workspace) fn ordered_groups(
        &self,
    ) -> impl Iterator<Item = &ProjectGroup> {
        self.groups
            .iter()
            .filter(|group| group.pinned)
            .chain(self.groups.iter().filter(|group| !group.pinned))
    }

    pub(in crate::gpui_shell::workspace) fn toggle_collapsed(
        &mut self,
        directory: &ProjectDirectory,
    ) {
        if let Some(group) = self.groups.iter_mut().find(|group| group.directory == *directory) {
            group.collapsed = !group.collapsed;
        }
    }

    pub(in crate::gpui_shell::workspace) fn toggle_pinned(&mut self, directory: &ProjectDirectory) {
        if let Some(group) = self.groups.iter_mut().find(|group| group.directory == *directory) {
            group.pinned = !group.pinned;
        }
    }

    pub(in crate::gpui_shell::workspace) fn reveal(&mut self, pane: u64) {
        if let Some(group) = self.groups.iter_mut().find(|group| group.panes.contains(&pane)) {
            group.collapsed = false;
        }
    }

    pub(in crate::gpui_shell::workspace) fn pending_panes(&self) -> impl Iterator<Item = u64> + '_ {
        self.panes.iter().filter(|pane| !pane.grouped).map(|pane| pane.id)
    }

    pub(in crate::gpui_shell::workspace) fn needs_refresh(&self) -> bool {
        self.panes.iter().any(|pane| !pane.grouped && pane.project.directory().is_some())
    }

    /// Surviving entries retain their first-observed order regardless of tab order.
    /// Call after a structural operation has completed, never during a remove/insert move.
    pub(in crate::gpui_shell::workspace) fn sync(
        &mut self,
        current: impl IntoIterator<Item = (u64, PaneProject)>,
    ) {
        let current: Vec<_> = current.into_iter().collect();
        self.panes.retain(|pane| current.iter().any(|(id, _)| *id == pane.id));
        for (id, project) in current {
            if let Some(pane) = self.panes.iter_mut().find(|pane| pane.id == id) {
                pane.project = project;
            } else {
                self.panes.push(PaneEntry { id, project, grouped: false });
            }
        }
        self.groups.retain(|group| {
            self.panes.iter().any(|pane| pane.project.directory() == Some(&group.directory))
        });
        for group in &mut self.groups {
            group.panes.clear();
        }
        for pane in &mut self.panes {
            pane.grouped = pane.project.directory().is_some();
            let Some(directory) = pane.project.directory() else { continue };
            if let Some(group) = self.groups.iter_mut().find(|group| group.directory == *directory)
            {
                group.panes.push(pane.id);
            } else {
                self.groups.push(ProjectGroup {
                    directory: directory.clone(),
                    panes: vec![pane.id],
                    collapsed: false,
                    pinned: false,
                });
            }
        }
    }
}

#[cfg(test)]
#[path = "model/tests.rs"]
mod tests;
