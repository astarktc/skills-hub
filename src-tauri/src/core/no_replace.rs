//! Renaming an entry within one directory without ever replacing what the new
//! name already holds — how a new Library copy is published
//! (`install_finalize`).
//!
//! `std::fs::rename` replaces: on Unix `rename(2)` puts a directory over an
//! empty directory, and on Windows `std` asks for replace-if-exists. A check
//! that the new name is free, followed by such a rename, leaves a window in
//! which something put there is replaced. Here the kernel makes the check and
//! the new entry in one step, per OS:
//!
//! - **Linux**: `renameat2(.., RENAME_NOREPLACE)` (kernel 3.15), called as the
//!   raw syscall so no glibc version is required: `EEXIST` when anything —
//!   a link included, never followed — is at the new name.
//! - **macOS**: `renameatx_np(.., RENAME_EXCL)` (10.12): the same contract.
//! - **Windows**: `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`, which
//!   fails with `ERROR_ALREADY_EXISTS` when anything is at the new name, on
//!   every volume; without `MOVEFILE_COPY_ALLOWED` it never copies either.
//!
//! Both names are resolved in one **held** directory ([`HeldDir`]). On Unix
//! the rename is relative to the held directory's descriptor, so it happens
//! in the directory that was opened, whatever its path names later. On
//! Windows the rename is by path, and the held handle shares no delete
//! access, so the directory cannot be renamed or removed while it is held.
//!
//! Where the file system has no exclusive rename (Linux `EINVAL` / `ENOSYS`:
//! NFS, CIFS, older FUSE; macOS `ENOTSUP` / `EOPNOTSUPP` / `EINVAL`: SMB,
//! exFAT, …), the new name is
//! reserved by an exclusive `mkdirat` — which fails on any entry, a dangling
//! link included, following none — and the rename lands on that reservation.
//! A reservation alone is not the primitive, only this fallback: POSIX
//! `rename` of a directory never follows a link at the new name and never
//! replaces a non-empty directory or a non-directory, so what remains is an
//! empty directory put in place of the reservation after it was made, which
//! would be replaced (no content lost, nothing written elsewhere). Windows
//! needs no fallback, and could not use one: a directory cannot be renamed
//! onto an existing one there.
//!
//! [`HeldDir::rename_no_replace_strict`] is the same rename without that
//! fallback: where the file system has no exclusive rename it says so
//! ([`StrictRenameError::Unsupported`]) and changes nothing. The redeploy
//! uses it for every rename (`core::redeploy`): an interruption between the
//! reservation and the rename would leave an empty folder its journal cannot
//! tell from anything else (round 20 t10 review, finding 8).
//!
//! [`HeldDir::exchange`] swaps two entries of the held directory in one step
//! where the kernel offers it (Linux `renameat2(.., RENAME_EXCHANGE)`, macOS
//! `renameatx_np(.., RENAME_SWAP)`), so neither name is ever empty — how the
//! redeploy replaces a Deployment recoverably (`core::redeploy`). Elsewhere
//! (Windows, a file system without it) it says so, and the caller decides
//! whether two strict renames under its own journal are safe.
//!
//! Scope (CONTEXT.md, Artifact removal): another process swapping entries
//! inside the Library while the app acts is out of scope. This module closes
//! the app's own check-then-act gaps; hardening against a concurrent
//! same-user process is BACKLOG #119.

use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Why [`HeldDir::rename_no_replace`] did not rename.
#[derive(Debug)]
pub(crate) enum RenameError {
    /// Something is at the new name (of any type, a link included). Nothing
    /// changed.
    Occupied,
    /// Any other failure. Nothing was put at the new name.
    Failed(io::Error),
}

/// Why [`HeldDir::rename_no_replace_strict`] did not rename. Nothing changed
/// in any case.
#[derive(Debug)]
pub(crate) enum StrictRenameError {
    /// Something is at the new name (of any type, a link included).
    Occupied,
    /// No exclusive rename on this file system; no fallback was tried.
    /// (Never on Windows, whose rename is exclusive on every volume.)
    #[cfg_attr(windows, allow(dead_code))]
    Unsupported,
    /// Any other failure.
    Failed(io::Error),
}

/// Why [`HeldDir::exchange`] did not exchange.
#[derive(Debug)]
pub(crate) enum ExchangeError {
    /// No atomic exchange on this OS or file system. Nothing changed.
    Unsupported,
    /// Any other failure (a name absent included). Nothing changed.
    Failed(io::Error),
}

/// A directory held open while entries are renamed within it.
#[derive(Debug)]
pub(crate) struct HeldDir {
    // Held for its descriptor (Unix) or for the share lock it holds (Windows).
    #[cfg_attr(windows, allow(dead_code))]
    dir: File,
    path: PathBuf,
}

impl HeldDir {
    /// Open the directory `path`, following links on the way and at its leaf
    /// (a Library root may be reached through an alias).
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        Ok(Self {
            dir: sys::open(path)?,
            path: path.to_path_buf(),
        })
    }

    /// Rename this directory's entry `from` to `to`, never replacing an entry
    /// at `to`. Both are single folder names.
    pub(crate) fn rename_no_replace(&self, from: &OsStr, to: &OsStr) -> Result<(), RenameError> {
        for name in [from, to] {
            let mut components = Path::new(name).components();
            if !matches!(
                (components.next(), components.next()),
                (Some(Component::Normal(_)), None)
            ) {
                return Err(RenameError::Failed(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{name:?} is not one entry name"),
                )));
            }
        }
        sys::rename_no_replace(self, from, to)
    }

    /// [`Self::rename_no_replace`] without the reservation fallback: where
    /// the file system has no exclusive rename,
    /// [`StrictRenameError::Unsupported`] and nothing changed — no entry is
    /// ever made at `to` but the renamed one. Both are single folder names.
    pub(crate) fn rename_no_replace_strict(
        &self,
        from: &OsStr,
        to: &OsStr,
    ) -> Result<(), StrictRenameError> {
        for name in [from, to] {
            let mut components = Path::new(name).components();
            if !matches!(
                (components.next(), components.next()),
                (Some(Component::Normal(_)), None)
            ) {
                return Err(StrictRenameError::Failed(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{name:?} is not one entry name"),
                )));
            }
        }
        sys::rename_no_replace_strict(self, from, to)
    }

    /// Exchange this directory's entries `a` and `b` (both present, of any
    /// type) in one atomic step, where the OS and file system offer it;
    /// [`ExchangeError::Unsupported`] elsewhere. Both are single folder names.
    pub(crate) fn exchange(&self, a: &OsStr, b: &OsStr) -> Result<(), ExchangeError> {
        for name in [a, b] {
            let mut components = Path::new(name).components();
            if !matches!(
                (components.next(), components.next()),
                (Some(Component::Normal(_)), None)
            ) {
                return Err(ExchangeError::Failed(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{name:?} is not one entry name"),
                )));
            }
        }
        sys::exchange(self, a, b)
    }
}

#[cfg(unix)]
mod sys {
    use std::ffi::{CString, OsStr};
    use std::fs::File;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::io::{AsRawFd, RawFd};
    use std::path::Path;

    use super::{ExchangeError, HeldDir, RenameError, StrictRenameError};

    pub(super) fn open(path: &Path) -> io::Result<File> {
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY)
            .open(path)
    }

    pub(super) fn rename_no_replace(
        dir: &HeldDir,
        from: &OsStr,
        to: &OsStr,
    ) -> Result<(), RenameError> {
        let fd = dir.dir.as_raw_fd();
        let from = c_name(from).map_err(RenameError::Failed)?;
        let to = c_name(to).map_err(RenameError::Failed)?;
        match exclusive_rename(fd, &from, &to) {
            Ok(()) => Ok(()),
            Err(err) if unsupported(&err) => {
                log::warn!(
                    "[publish] no exclusive rename in {:?} ({err}); reserving the name instead",
                    dir.path
                );
                rename_onto_reservation(fd, &from, &to)
            }
            Err(err) => Err(classify(err)),
        }
    }

    pub(super) fn rename_no_replace_strict(
        dir: &HeldDir,
        from: &OsStr,
        to: &OsStr,
    ) -> Result<(), StrictRenameError> {
        let fd = dir.dir.as_raw_fd();
        let from = c_name(from).map_err(StrictRenameError::Failed)?;
        let to = c_name(to).map_err(StrictRenameError::Failed)?;
        match exclusive_rename(fd, &from, &to) {
            Ok(()) => Ok(()),
            Err(err) if unsupported(&err) => Err(StrictRenameError::Unsupported),
            Err(err) => Err(match classify(err) {
                RenameError::Occupied => StrictRenameError::Occupied,
                RenameError::Failed(err) => StrictRenameError::Failed(err),
            }),
        }
    }

    pub(super) fn exchange(dir: &HeldDir, a: &OsStr, b: &OsStr) -> Result<(), ExchangeError> {
        let fd = dir.dir.as_raw_fd();
        let a = c_name(a).map_err(ExchangeError::Failed)?;
        let b = c_name(b).map_err(ExchangeError::Failed)?;
        match atomic_exchange(fd, &a, &b) {
            Ok(()) => Ok(()),
            Err(err) if unsupported(&err) => Err(ExchangeError::Unsupported),
            Err(err) => Err(ExchangeError::Failed(err)),
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn atomic_exchange(fd: RawFd, a: &CString, b: &CString) -> io::Result<()> {
        // SAFETY: a live directory descriptor and two NUL-terminated names;
        // every variadic argument is passed at register width.
        let rc = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                fd as libc::c_long,
                a.as_ptr(),
                fd as libc::c_long,
                b.as_ptr(),
                libc::RENAME_EXCHANGE as libc::c_long,
            )
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    #[cfg(target_vendor = "apple")]
    fn atomic_exchange(fd: RawFd, a: &CString, b: &CString) -> io::Result<()> {
        // SAFETY: a live directory descriptor and two NUL-terminated names.
        let rc = unsafe { libc::renameatx_np(fd, a.as_ptr(), fd, b.as_ptr(), libc::RENAME_SWAP) };
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
    fn atomic_exchange(_fd: RawFd, _a: &CString, _b: &CString) -> io::Result<()> {
        Err(io::Error::from_raw_os_error(libc::ENOSYS))
    }

    /// The exclusive-rename fallback: reserve `to` with an exclusive
    /// `mkdirat`, then rename onto that empty reservation (module docs).
    pub(super) fn rename_onto_reservation(
        fd: RawFd,
        from: &CString,
        to: &CString,
    ) -> Result<(), RenameError> {
        // SAFETY: a live directory descriptor and a NUL-terminated name.
        if unsafe { libc::mkdirat(fd, to.as_ptr(), 0o700) } != 0 {
            return Err(classify(io::Error::last_os_error()));
        }
        let reserved = entry_id(fd, to);
        // SAFETY: as above, for both names.
        if unsafe { libc::renameat(fd, from.as_ptr(), fd, to.as_ptr()) } == 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        // Remove the reservation only while it is still the one made here;
        // AT_REMOVEDIR removes nothing but an empty directory.
        if let Ok(reserved) = reserved {
            if entry_id(fd, to).is_ok_and(|now| now == reserved) {
                // SAFETY: as above.
                unsafe { libc::unlinkat(fd, to.as_ptr(), libc::AT_REMOVEDIR) };
            }
        }
        Err(classify(err))
    }

    pub(super) fn c_name(name: &OsStr) -> io::Result<CString> {
        CString::new(name.as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "name contains NUL"))
    }

    /// The device and inode of the entry `name` itself (a link not followed).
    fn entry_id(fd: RawFd, name: &CString) -> io::Result<(u64, u64)> {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: a live descriptor, a NUL-terminated name and a stat buffer
        // the call fills on success.
        let rc = unsafe {
            libc::fstatat(
                fd,
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fstatat succeeded, so the buffer is initialised.
        let stat = unsafe { stat.assume_init() };
        #[allow(clippy::unnecessary_cast)]
        Ok((stat.st_dev as u64, stat.st_ino as u64))
    }

    fn classify(err: io::Error) -> RenameError {
        match err.raw_os_error() {
            Some(libc::EEXIST | libc::ENOTEMPTY | libc::ENOTDIR | libc::EISDIR) => {
                RenameError::Occupied
            }
            _ => RenameError::Failed(err),
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn exclusive_rename(fd: RawFd, from: &CString, to: &CString) -> io::Result<()> {
        // SAFETY: a live directory descriptor and two NUL-terminated names;
        // every variadic argument is passed at register width.
        let rc = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                fd as libc::c_long,
                from.as_ptr(),
                fd as libc::c_long,
                to.as_ptr(),
                libc::RENAME_NOREPLACE as libc::c_long,
            )
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn unsupported(err: &io::Error) -> bool {
        matches!(err.raw_os_error(), Some(libc::EINVAL | libc::ENOSYS))
    }

    #[cfg(target_vendor = "apple")]
    fn exclusive_rename(fd: RawFd, from: &CString, to: &CString) -> io::Result<()> {
        // SAFETY: a live directory descriptor and two NUL-terminated names.
        let rc =
            unsafe { libc::renameatx_np(fd, from.as_ptr(), fd, to.as_ptr(), libc::RENAME_EXCL) };
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    #[cfg(target_vendor = "apple")]
    fn unsupported(err: &io::Error) -> bool {
        matches!(
            err.raw_os_error(),
            Some(libc::ENOTSUP | libc::EOPNOTSUPP | libc::EINVAL)
        )
    }

    #[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
    fn exclusive_rename(_fd: RawFd, _from: &CString, _to: &CString) -> io::Result<()> {
        Err(io::Error::from_raw_os_error(libc::ENOSYS))
    }

    #[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
    fn unsupported(err: &io::Error) -> bool {
        err.raw_os_error() == Some(libc::ENOSYS)
    }
}

#[cfg(windows)]
mod sys {
    use std::ffi::OsStr;
    use std::fs::File;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::OpenOptionsExt;
    use std::path::Path;

    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS};
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    use super::{ExchangeError, HeldDir, RenameError, StrictRenameError};

    /// Windows has no atomic exchange of two directory entries.
    pub(super) fn exchange(_dir: &HeldDir, _a: &OsStr, _b: &OsStr) -> Result<(), ExchangeError> {
        Err(ExchangeError::Unsupported)
    }

    pub(super) fn open(path: &Path) -> io::Result<File> {
        const FILE_READ_ATTRIBUTES: u32 = 0x0080;
        const FILE_SHARE_READ: u32 = 0x1;
        const FILE_SHARE_WRITE: u32 = 0x2;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        // No FILE_SHARE_DELETE: while held, the directory cannot be renamed
        // or removed. Renaming entries inside it needs no delete access to it.
        std::fs::OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
    }

    pub(super) fn rename_no_replace(
        dir: &HeldDir,
        from: &OsStr,
        to: &OsStr,
    ) -> Result<(), RenameError> {
        let wide = |name: &OsStr| -> Vec<u16> {
            dir.path
                .join(name)
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect()
        };
        let (from, to) = (wide(from), wide(to));
        // SAFETY: two NUL-terminated wide paths. Flags 0: no replace, no copy.
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) } != 0 {
            return Ok(());
        }
        // SAFETY: reads this thread's last error.
        match unsafe { GetLastError() } {
            ERROR_ALREADY_EXISTS | ERROR_FILE_EXISTS => Err(RenameError::Occupied),
            code => Err(RenameError::Failed(io::Error::from_raw_os_error(
                code as i32,
            ))),
        }
    }

    /// `MoveFileExW` without replace is exclusive on every volume: no
    /// fallback exists to refuse, so this is the ordinary rename.
    pub(super) fn rename_no_replace_strict(
        dir: &HeldDir,
        from: &OsStr,
        to: &OsStr,
    ) -> Result<(), StrictRenameError> {
        rename_no_replace(dir, from, to).map_err(|err| match err {
            RenameError::Occupied => StrictRenameError::Occupied,
            RenameError::Failed(err) => StrictRenameError::Failed(err),
        })
    }
}

#[cfg(test)]
#[path = "tests/no_replace.rs"]
mod tests;
