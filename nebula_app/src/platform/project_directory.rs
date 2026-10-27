//! Lexical local-directory identity. No filesystem I/O or symlink resolution.

use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct ProjectDirectory {
    path: PathBuf,
    #[cfg(windows)]
    ordinal: Vec<u16>,
}

impl ProjectDirectory {
    pub(crate) fn new(path: &Path) -> Option<Self> {
        let raw = path.to_str()?;
        if raw.chars().any(char::is_control) || raw.encode_utf16().count() > 32767 {
            return None;
        }
        #[cfg(windows)]
        let (path, verbatim) = {
            let raw = raw.replace('/', "\\");
            let verbatim = raw.starts_with(r"\\?\");
            let identity = if raw.get(..8).is_some_and(|s| s.eq_ignore_ascii_case(r"\\?\UNC\")) {
                format!(r"\\{}", &raw[8..])
            } else if verbatim {
                raw[4..].to_owned()
            } else {
                raw
            };
            (PathBuf::from(identity), verbatim)
        };
        if !path.is_absolute() {
            return None;
        }
        #[cfg(windows)]
        match path.components().next()? {
            Component::Prefix(prefix)
                if matches!(
                    prefix.kind(),
                    std::path::Prefix::Disk(_) | std::path::Prefix::UNC(..)
                ) => {},
            _ => return None,
        }
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {},
                Component::ParentDir => {
                    normalized.pop();
                },
                component => normalized.push(component.as_os_str()),
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt as _;
            let ordinal = normalized.as_os_str().encode_wide().collect();
            if verbatim {
                let raw = normalized.to_str()?;
                normalized = if let Some(unc) = raw.strip_prefix(r"\\") {
                    PathBuf::from(format!(r"\\?\UNC\{unc}"))
                } else {
                    PathBuf::from(format!(r"\\?\{raw}"))
                };
            }
            Some(Self { path: normalized, ordinal })
        }
        #[cfg(not(windows))]
        Some(Self { path: normalized })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl PartialEq for ProjectDirectory {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};
            // SAFETY: both slices contain initialized UTF-16, stay alive for the call,
            // and their lengths are bounded at construction. No terminator is required.
            unsafe {
                CompareStringOrdinal(
                    self.ordinal.as_ptr(),
                    self.ordinal.len() as i32,
                    other.ordinal.as_ptr(),
                    other.ordinal.len() as i32,
                    1,
                ) == CSTR_EQUAL
            }
        }
        #[cfg(not(windows))]
        {
            self.path == other.path
        }
    }
}

impl Eq for ProjectDirectory {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_absolute_directories_without_control_characters_are_candidates() {
        for raw in ["", "relative", "../project", "C:relative", "C:\\work\nproject"] {
            assert!(ProjectDirectory::new(Path::new(raw)).is_none(), "{raw:?}");
        }
        let root = std::env::temp_dir();
        assert_eq!(
            ProjectDirectory::new(&root.join("one/../two/.")).unwrap(),
            ProjectDirectory::new(&root.join("two")).unwrap()
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_identity_normalizes_spelling_without_merging_distinct_paths() {
        let directory = |raw: &str| ProjectDirectory::new(Path::new(raw)).unwrap();
        for raw in [r"c:/WORK/demo/", r"C:\work\.\other\..\demo", r"\\?\C:\work\demo"] {
            assert_eq!(directory(raw), directory(r"C:\work\demo"));
        }
        assert_eq!(directory(r"C:\Équipe\项目"), directory(r"c:\équipe\项目"));
        assert_ne!(directory(r"C:\work\demo"), directory(r"D:\work\demo"));
        assert_ne!(directory(r"C:\work\demo"), directory(r"C:\other\demo"));
        assert_ne!(directory(r"C:\Straße"), directory(r"C:\Strasse"));
        assert_eq!(directory(r"\\?\UNC\server\share\demo"), directory(r"\\SERVER\share\demo\"));
        assert_eq!(directory(r"\\?\C:\work\demo").path(), Path::new(r"\\?\C:\work\demo"));
        for raw in [r"\relative", r"\\.\pipe\test", r"\\?\GLOBALROOT\Device\test"] {
            assert!(ProjectDirectory::new(Path::new(raw)).is_none());
        }
    }
}
