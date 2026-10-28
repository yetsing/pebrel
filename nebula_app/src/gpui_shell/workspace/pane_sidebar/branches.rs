//! Window-owned, bounded branch snapshots of the fixed project directories.

use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

struct Entry {
    directory: ProjectDirectory,
    generation: u64,
    branch: Option<String>,
    checked: Option<Instant>,
}

struct Read {
    _task: gpui::Task<()>,
    cancelled: Arc<AtomicBool>,
}

impl Drop for Read {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

#[derive(Default)]
pub(super) struct Branches {
    entries: Vec<Entry>,
    next_generation: u64,
    pending: Option<Read>,
}

impl Branches {
    #[cfg(all(test, feature = "gpui-test-support"))]
    pub(super) fn set_for_test(&mut self, directory: &ProjectDirectory, branch: &str) {
        let generation =
            self.entries.iter().find(|entry| entry.directory == *directory).unwrap().generation;
        self.publish(generation, Some(branch.into()));
    }
    pub(super) fn sync<'a>(&mut self, directories: impl Iterator<Item = &'a ProjectDirectory>) {
        let directories = directories.collect::<Vec<_>>();
        let changed = self.entries.len() != directories.len()
            || self.entries.iter().any(|entry| !directories.contains(&&entry.directory));
        if !changed {
            return;
        }
        self.pending = None;
        self.entries.retain(|entry| directories.contains(&&entry.directory));
        for entry in &mut self.entries {
            entry.checked = None;
        }
        for directory in directories {
            if !self.entries.iter().any(|entry| entry.directory == *directory) {
                self.next_generation += 1;
                self.entries.push(Entry {
                    directory: directory.clone(),
                    generation: self.next_generation,
                    branch: None,
                    checked: None,
                });
            }
        }
    }

    pub(super) fn get(&self, directory: &ProjectDirectory) -> Option<&str> {
        self.entries.iter().find(|entry| entry.directory == *directory)?.branch.as_deref()
    }

    fn requests(&mut self, now: Instant) -> Vec<(u64, ProjectDirectory)> {
        let mut due = self
            .entries
            .iter_mut()
            .filter(|entry| {
                entry.checked.is_none_or(|at| now.duration_since(at) >= Duration::from_secs(5))
            })
            .collect::<Vec<_>>();
        // Slow repositories must not keep later groups from receiving a first snapshot.
        due.sort_by_key(|entry| entry.checked);
        due.into_iter()
            .take(4)
            .map(|entry| {
                entry.checked = Some(now);
                (entry.generation, entry.directory.clone())
            })
            .collect()
    }

    fn publish(&mut self, generation: u64, branch: Option<String>) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.generation == generation)
        else {
            return false;
        };
        if entry.branch == branch {
            return false;
        }
        entry.branch = branch;
        true
    }
}

impl NebulaWorkspace {
    pub(in crate::gpui_shell::workspace) fn refresh_sidebar_branches(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        #[cfg(all(test, feature = "gpui-test-support"))]
        if cx.try_global::<shell_launch::TestConfiguredLaunch>().is_some() {
            return;
        }
        if !cfg!(windows)
            || self.tabs_position != nebula_settings::TabsPositionName::Sidebar
            || self.sidebar_collapsed
            || self.tabs_section_collapsed
        {
            self.pane_sidebar.branches.pending = None;
            return;
        }
        let branches = &mut self.pane_sidebar.branches;
        if branches.pending.is_some() {
            return;
        }
        let requests = branches.requests(Instant::now());
        if requests.is_empty() {
            return;
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let stop = cancelled.clone();
        let work = cx.background_executor().spawn(async move {
            let mut results = Vec::new();
            for (generation, directory) in requests {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let branch = crate::display::side_panel::read_git_branch(directory.path(), &|| {
                    stop.load(Ordering::Relaxed)
                });
                results.push((generation, branch));
            }
            results
        });
        let task = cx.spawn(async move |this, cx| {
            let results = work.await;
            let _ = this.update(cx, |workspace, cx| {
                let branches = &mut workspace.pane_sidebar.branches;
                branches.pending = None;
                let mut changed = false;
                for (generation, branch) in results {
                    changed |= branches.publish(generation, branch);
                }
                if changed {
                    cx.notify();
                }
            });
        });
        self.pane_sidebar.branches.pending = Some(Read { _task: task, cancelled });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_batches_do_not_starve_later_groups_and_snapshots_are_cached() {
        let directories = (0..7)
            .map(|index| {
                ProjectDirectory::new(&std::env::temp_dir().join(format!("branch-{index}")))
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let mut branches = Branches::default();
        branches.sync(directories.iter());
        let now = Instant::now();
        let first = branches.requests(now);
        assert_eq!(
            first.iter().map(|(_, dir)| dir).collect::<Vec<_>>(),
            directories[..4].iter().collect::<Vec<_>>()
        );
        // A batch of four reads may take longer than the refresh interval.
        let later = now + Duration::from_secs(8);
        let second = branches.requests(later);
        assert_eq!(
            second.iter().take(3).map(|(_, dir)| dir).collect::<Vec<_>>(),
            directories[4..].iter().collect::<Vec<_>>()
        );
        assert_eq!(second.len(), 4);
        assert_eq!(branches.requests(later).len(), 3);
        assert!(branches.requests(later + Duration::from_secs(4)).is_empty());
        assert_eq!(branches.requests(later + Duration::from_secs(5)).len(), 4);
    }

    #[test]
    fn closed_and_recreated_project_rejects_the_old_branch_result() {
        let directory = ProjectDirectory::new(&std::env::temp_dir().join("branch-owner")).unwrap();
        let mut branches = Branches::default();
        branches.sync(std::iter::once(&directory));
        let old = branches.entries[0].generation;
        assert!(branches.publish(old, Some("old".into())));
        branches.sync(std::iter::empty());
        branches.sync(std::iter::once(&directory));
        assert!(!branches.publish(old, Some("late".into())));
        assert!(branches.get(&directory).is_none());
        let current = branches.entries[0].generation;
        assert!(branches.publish(current, Some("new".into())));
        assert_eq!(branches.get(&directory), Some("new"));
        assert!(branches.publish(current, None));
        assert!(branches.get(&directory).is_none());
    }
}
