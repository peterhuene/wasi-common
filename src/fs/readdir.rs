use crate::fs::DirEntry;
use crate::{host, hostcalls};

/// Iterator over the entries in a directory.
///
/// This corresponds to [`std::fs::ReadDir`].
///
/// TODO: Not yet implemented.
///
/// [`std::fs::ReadDir`]: https://doc.rust-lang.org/std/fs/struct.ReadDir.html
pub struct ReadDir {
    fd: host::__wasi_fd_t,
}

impl ReadDir {
    /// Constructs a new instance of `Self` from the given raw WASI file descriptor.
    pub unsafe fn from_raw_wasi_fd(fd: host::__wasi_fd_t) -> Self {
        Self { fd }
    }
}

/// TODO: Not yet implemented.
impl Iterator for ReadDir {
    type Item = DirEntry;

    /// TODO: Not yet implemented.
    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!("ReadDir::next");
    }
}

// TODO: impl Debug for ReadDir
