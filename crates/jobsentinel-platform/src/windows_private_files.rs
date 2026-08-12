// Enforces the closed Windows ACL policy for app-owned private files and directories.
#![allow(unsafe_code)]

mod platform {
    use crate::{
        windows_acl_policy::{private_acl_policy, AceSpec, PrivateObjectKind, TrusteeRole},
        windows_private_path::open_file as lock_file,
    };
    use std::{
        fs::File,
        io,
        mem::size_of,
        os::windows::io::{AsRawHandle, FromRawHandle},
        path::Path,
        ptr::{null, null_mut},
    };
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, LocalFree, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, HANDLE,
            INVALID_HANDLE_VALUE,
        },
        Security::{
            Authorization::{
                BuildTrusteeWithSidW, GetSecurityInfo, SetEntriesInAclW, SetSecurityInfo,
                EXPLICIT_ACCESS_W, SET_ACCESS, SE_FILE_OBJECT,
            },
            CreateWellKnownSid, EqualSid, GetTokenInformation, IsValidSid, TokenUser,
            WinBuiltinAdministratorsSid, WinLocalSystemSid, ACL, DACL_SECURITY_INFORMATION,
            OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSID,
            SECURITY_MAX_SID_SIZE, TOKEN_QUERY, TOKEN_USER, WELL_KNOWN_SID_TYPE,
        },
        Storage::FileSystem::{
            ReOpenFile, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_SHARE_READ,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: this wrapper owns a real token handle returned by OpenProcessToken.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    struct LocalMemory(*mut core::ffi::c_void);

    impl Drop for LocalMemory {
        fn drop(&mut self) {
            // SAFETY: a Windows security API returned this LocalAlloc-owned block.
            unsafe {
                LocalFree(self.0);
            }
        }
    }

    pub(crate) fn ensure_private_file(path: &Path) -> io::Result<()> {
        let locked = match lock_file(path) {
            Ok(locked) => locked,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        apply_private_dacl(locked.object(), PrivateObjectKind::File)
    }

    pub(crate) fn open_private_file(path: &Path) -> io::Result<Option<File>> {
        let locked = match lock_file(path) {
            Ok(locked) => locked,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        apply_private_dacl(locked.object(), PrivateObjectKind::File)?;
        // SAFETY: the validated source handle remains live and no-delete shared for this call.
        let handle = unsafe {
            ReOpenFile(
                object_handle(locked.object()),
                FILE_GENERIC_READ,
                FILE_SHARE_READ,
                FILE_FLAG_OPEN_REPARSE_POINT,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: ReOpenFile returned a new owned handle that is transferred exactly once to File.
        Ok(Some(unsafe { File::from_raw_handle(handle) }))
    }

    fn validate_owner(object: &File, allowed: [PSID; 3]) -> io::Result<()> {
        let mut owner = null_mut();
        let mut descriptor = null_mut();
        // SAFETY: object is live with READ_CONTROL; owner and descriptor are initialized
        // out-pointers, while unrequested group and ACL outputs may be null.
        let result = unsafe {
            GetSecurityInfo(
                object_handle(object),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION,
                &mut owner,
                null_mut(),
                null_mut(),
                null_mut(),
                &mut descriptor,
            )
        };
        if result != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(result.cast_signed()));
        }
        if descriptor.is_null() {
            return Err(invalid_data("private storage descriptor is missing"));
        }
        let _descriptor = LocalMemory(descriptor);
        if owner.is_null() {
            return Err(invalid_data("private storage owner is missing"));
        }
        // SAFETY: owner borrows from the live descriptor and allowed SIDs borrow from live buffers.
        if allowed
            .iter()
            .any(|sid| unsafe { EqualSid(owner, *sid) } != 0)
        {
            Ok(())
        } else {
            Err(invalid_data("private storage owner is not trusted"))
        }
    }

    fn object_handle(object: &File) -> HANDLE {
        object.as_raw_handle().cast::<core::ffi::c_void>() as HANDLE
    }

    pub(crate) fn apply_private_dacl(object: &File, kind: PrivateObjectKind) -> io::Result<()> {
        let mut current_user_buffer = current_user_sid()?;
        let mut local_system_buffer = well_known_sid(WinLocalSystemSid)?;
        let mut administrators_buffer = well_known_sid(WinBuiltinAdministratorsSid)?;
        let current_user = token_user_sid(&mut current_user_buffer)?;
        let local_system = sid_pointer(&mut local_system_buffer)?;
        let administrators = sid_pointer(&mut administrators_buffer)?;
        validate_owner(object, [current_user, local_system, administrators])?;
        let policy = private_acl_policy(kind);
        let entries = policy.map(|spec| {
            let sid = match spec.trustee {
                TrusteeRole::CurrentUser => current_user,
                TrusteeRole::LocalSystem => local_system,
                TrusteeRole::BuiltinAdministrators => administrators,
            };
            explicit_access(spec, sid)
        });
        let mut acl = null_mut();
        // SAFETY: entries and all referenced SID buffers remain alive through this call; old ACL
        // is null so the API creates a new LocalAlloc-owned ACL in the initialized out-pointer.
        let result =
            unsafe { SetEntriesInAclW(entries.len() as u32, entries.as_ptr(), null(), &mut acl) };
        if result != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(result.cast_signed()));
        }
        if acl.is_null() {
            return Err(invalid_data("private storage DACL is missing"));
        }
        let acl = LocalMemory(acl.cast());

        // SAFETY: object owns a live file-system handle opened with WRITE_DAC; dacl borrows from
        // acl, which remains alive through SetSecurityInfo; null SID/SACL pointers are permitted
        // because only DACL flags are requested.
        let result = unsafe {
            SetSecurityInfo(
                object_handle(object),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                acl.0.cast::<ACL>(),
                null(),
            )
        };
        if result == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(result.cast_signed()))
        }
    }

    fn explicit_access(spec: AceSpec, sid: PSID) -> EXPLICIT_ACCESS_W {
        let mut trustee = windows_sys::Win32::Security::Authorization::TRUSTEE_W::default();
        // SAFETY: sid was validated and its backing buffer outlives the returned entry's use.
        unsafe {
            BuildTrusteeWithSidW(&mut trustee, sid);
        }
        EXPLICIT_ACCESS_W {
            grfAccessPermissions: spec.access_mask,
            grfAccessMode: SET_ACCESS,
            grfInheritance: spec.inheritance,
            Trustee: trustee,
        }
    }

    fn current_user_sid() -> io::Result<Vec<usize>> {
        let mut token = null_mut();
        // SAFETY: GetCurrentProcess returns a process pseudo-handle; token is an initialized
        // out-pointer and receives an owned handle only on success.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let _token = OwnedHandle(token);
        let mut byte_len = 0;
        // SAFETY: the null-buffer probe is the documented sizing call for TokenUser.
        if unsafe { GetTokenInformation(token, TokenUser, null_mut(), 0, &mut byte_len) } != 0 {
            return Err(invalid_data("Windows token sizing unexpectedly succeeded"));
        }
        let sizing_error = io::Error::last_os_error();
        if sizing_error.raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER.cast_signed())
            || byte_len < size_of::<TOKEN_USER>() as u32
        {
            return Err(sizing_error);
        }
        let word_len = (byte_len as usize).div_ceil(size_of::<usize>());
        let mut buffer = vec![0usize; word_len];
        // SAFETY: the usize buffer is suitably aligned and contains at least byte_len writable
        // bytes; token stays live throughout the call.
        if unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                byte_len,
                &mut byte_len,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        token_user_sid(&mut buffer)?;
        Ok(buffer)
    }

    fn well_known_sid(kind: WELL_KNOWN_SID_TYPE) -> io::Result<Vec<usize>> {
        let mut byte_len = SECURITY_MAX_SID_SIZE;
        let word_len = (byte_len as usize).div_ceil(size_of::<usize>());
        let mut buffer = vec![0usize; word_len];
        // SAFETY: buffer is suitably aligned and has SECURITY_MAX_SID_SIZE writable bytes;
        // a null domain SID is documented for universal and well-known Windows SIDs.
        if unsafe {
            CreateWellKnownSid(kind, null_mut(), buffer.as_mut_ptr().cast(), &mut byte_len)
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        sid_pointer(&mut buffer)?;
        Ok(buffer)
    }

    fn token_user_sid(buffer: &mut [usize]) -> io::Result<PSID> {
        // SAFETY: current_user_sid sizes and fills this suitably aligned buffer as TOKEN_USER.
        let sid = unsafe { (*buffer.as_mut_ptr().cast::<TOKEN_USER>()).User.Sid };
        validate_sid(sid)
    }

    fn sid_pointer(buffer: &mut [usize]) -> io::Result<PSID> {
        validate_sid(buffer.as_mut_ptr().cast())
    }

    fn validate_sid(sid: PSID) -> io::Result<PSID> {
        // SAFETY: callers provide pointers into live token or fixed-size SID buffers.
        if sid.is_null() || unsafe { IsValidSid(sid) } == 0 {
            Err(invalid_data(
                "Windows returned an invalid security identifier",
            ))
        } else {
            Ok(sid)
        }
    }

    fn invalid_data(message: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, message)
    }
}

pub(crate) use platform::{apply_private_dacl, ensure_private_file, open_private_file};
