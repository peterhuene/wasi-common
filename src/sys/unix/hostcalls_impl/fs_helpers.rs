#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use crate::sys::host_impl;
use crate::{host, Result};
use std::fs::File;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub(crate) use super::super::linux::fs_helpers::*;
    } else if #[cfg(any(
            target_os = "macos",
            target_os = "netbsd",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "ios",
            target_os = "dragonfly"
    ))] {
        pub(crate) use super::super::bsd::fs_helpers::*;
    }
}

pub(crate) fn path_open_rights(
    rights_base: host::__wasi_rights_t,
    rights_inheriting: host::__wasi_rights_t,
    oflags: host::__wasi_oflags_t,
    fs_flags: host::__wasi_fdflags_t,
) -> (host::__wasi_rights_t, host::__wasi_rights_t) {
    use nix::fcntl::OFlag;

    // which rights are needed on the dirfd?
    let mut needed_base = host::__WASI_RIGHT_PATH_OPEN;
    let mut needed_inheriting = rights_base | rights_inheriting;

    // convert open flags
    let oflags = host_impl::nix_from_oflags(oflags);
    if oflags.contains(OFlag::O_CREAT) {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    }
    if oflags.contains(OFlag::O_TRUNC) {
        needed_base |= host::__WASI_RIGHT_PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    let fdflags = host_impl::nix_from_fdflags(fs_flags);
    if fdflags.contains(OFlag::O_DSYNC) {
        needed_inheriting |= host::__WASI_RIGHT_FD_DATASYNC;
    }
    if fdflags.intersects(host_impl::O_RSYNC | OFlag::O_SYNC) {
        needed_inheriting |= host::__WASI_RIGHT_FD_SYNC;
    }

    (needed_base, needed_inheriting)
}

pub(crate) fn openat(dirfd: &File, path: &str) -> Result<File> {
    use nix::fcntl::{self, OFlag};
    use nix::sys::stat::Mode;
    use std::os::unix::prelude::{AsRawFd, FromRawFd};

    log::debug!("path_get openat path = {:?}", path);

    fcntl::openat(
        dirfd.as_raw_fd(),
        path,
        OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW,
        Mode::empty(),
    )
    .map(|new_fd| unsafe { File::from_raw_fd(new_fd) })
    .map_err(Into::into)
}

pub(crate) fn readlinkat(dirfd: &File, path: &str) -> Result<String> {
    use nix::fcntl;
    use std::os::unix::prelude::AsRawFd;

    log::debug!("path_get readlinkat path = {:?}", path);

    let readlink_buf = &mut [0u8; libc::PATH_MAX as usize + 1];

    fcntl::readlinkat(dirfd.as_raw_fd(), path, readlink_buf)
        .map_err(Into::into)
        .and_then(host_impl::path_from_host)
}
