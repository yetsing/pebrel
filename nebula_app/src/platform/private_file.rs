//! 凭据临时文件权限的原生边界；原子写入与回滚仍由 atomic_file 持有。
use std::fs::OpenOptions;

pub(crate) fn restrict_creation(options: &mut OpenOptions) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = options;
}

/// Create an owner/SYSTEM-only temporary file with kernel-enforced crash cleanup.
#[cfg(windows)]
pub(crate) fn create_ephemeral(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    use std::os::windows::ffi::OsStrExt as _;
    use std::os::windows::io::FromRawHandle as _;
    use windows_sys::Win32::Foundation::{
        GENERIC_READ, GENERIC_WRITE, GetLastError, INVALID_HANDLE_VALUE, LocalFree,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
    use windows_sys::Win32::Storage::FileSystem::{
        CREATE_NEW, CreateFileW, DELETE, FILE_ATTRIBUTE_TEMPORARY, FILE_FLAG_DELETE_ON_CLOSE,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    let name = path.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
    let sddl = "D:P(A;;FA;;;SY)(A;;FA;;;OW)\0".encode_utf16().collect::<Vec<_>>();
    // CreateFileW copies the parsed descriptor before it is released.
    unsafe {
        let mut descriptor = std::ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        let handle = CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE | DELETE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            &attributes,
            CREATE_NEW,
            FILE_ATTRIBUTE_TEMPORARY | FILE_FLAG_DELETE_ON_CLOSE,
            std::ptr::null_mut(),
        );
        let error = GetLastError();
        LocalFree(descriptor);
        if handle == INVALID_HANDLE_VALUE {
            Err(std::io::Error::from_raw_os_error(error as i32))
        } else {
            Ok(std::fs::File::from_raw_handle(handle))
        }
    }
}

#[cfg(test)]
pub(crate) fn assert_private(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
    }
    #[cfg(not(unix))]
    assert!(path.is_file());
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn private_write_restricts_new_and_replaced_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        crate::atomic_file::write_private(&path, b"first").unwrap();
        assert_private(&path);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        crate::atomic_file::write_private(&path, b"replacement").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"replacement");
        assert_private(&path);
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    #[test]
    fn ephemeral_capture_is_private_from_creation_and_deleted_after_last_handle() {
        use std::os::windows::io::AsRawHandle as _;
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Authorization::{
            ConvertSecurityDescriptorToStringSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
            SE_FILE_OBJECT,
        };
        use windows_sys::Win32::Security::DACL_SECURITY_INFORMATION;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("capture.jsonl");
        let file = super::create_ephemeral(&path).unwrap();
        let duplicate = file.try_clone().unwrap();
        unsafe {
            let mut descriptor = std::ptr::null_mut();
            assert_eq!(
                GetSecurityInfo(
                    file.as_raw_handle(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut descriptor,
                ),
                0
            );
            let mut sddl = std::ptr::null_mut();
            let converted = ConvertSecurityDescriptorToStringSecurityDescriptorW(
                descriptor,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut sddl,
                std::ptr::null_mut(),
            );
            LocalFree(descriptor);
            assert_ne!(converted, 0);
            let mut length = 0;
            while *sddl.add(length) != 0 {
                length += 1;
            }
            let actual = String::from_utf16(std::slice::from_raw_parts(sddl, length)).unwrap();
            LocalFree(sddl.cast());
            assert_eq!(actual, "D:P(A;;FA;;;SY)(A;;FA;;;OW)");
        }
        drop(file);
        assert!(path.exists());
        drop(duplicate);
        assert!(!path.exists());
    }
}
