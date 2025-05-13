use core::{any::Any, ffi::c_int};

use alloc::{string::String, sync::Arc};
use axerrno::{LinuxError, LinuxResult};
use axio::PollState;
use axsync::{Mutex, MutexGuard};
use linux_raw_sys::general::{S_IFDIR, stat, statx};

use super::{add_file_like, get_file_like};

#[allow(dead_code)]
pub trait FileLike: Send + Sync {
    fn read(&self, buf: &mut [u8]) -> LinuxResult<usize>;
    fn write(&self, buf: &[u8]) -> LinuxResult<usize>;
    fn stat(&self) -> LinuxResult<Kstat>;
    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
    fn poll(&self) -> LinuxResult<PollState>;
    fn set_nonblocking(&self, nonblocking: bool) -> LinuxResult;

    fn from_fd(fd: c_int) -> LinuxResult<Arc<Self>>
    where
        Self: Sized + 'static,
    {
        get_file_like(fd)?
            .into_any()
            .downcast::<Self>()
            .map_err(|_| LinuxError::EINVAL)
    }

    fn add_to_fd_table(self) -> LinuxResult<c_int>
    where
        Self: Sized + 'static,
    {
        add_file_like(Arc::new(self))
    }
}

/// File wrapper for `axfs::fops::File`.
pub struct File {
    inner: Mutex<axfs::fops::File>,
    path: String,
}

impl File {
    pub fn new(inner: axfs::fops::File, path: String) -> Self {
        Self {
            inner: Mutex::new(inner),
            path,
        }
    }

    /// Get the path of the file.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the inner node of the file.
    pub fn inner(&self) -> MutexGuard<axfs::fops::File> {
        self.inner.lock()
    }
}

impl FileLike for File {
    fn read(&self, buf: &mut [u8]) -> LinuxResult<usize> {
        Ok(self.inner().read(buf)?)
    }

    fn write(&self, buf: &[u8]) -> LinuxResult<usize> {
        Ok(self.inner().write(buf)?)
    }

    fn stat(&self) -> LinuxResult<Kstat> {
        let metadata = self.inner().get_attr()?;
        let ty = metadata.file_type() as u8;
        let perm = metadata.perm().bits() as u32;

        Ok(Kstat {
            mode: ((ty as u32) << 12) | perm,
            size: metadata.size(),
            blocks: metadata.blocks(),
            blksize: 512,
            ..Default::default()
        })
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn poll(&self) -> LinuxResult<PollState> {
        Ok(PollState {
            readable: true,
            writable: true,
        })
    }

    fn set_nonblocking(&self, _nonblocking: bool) -> LinuxResult {
        Ok(())
    }
}

/// Directory wrapper for `axfs::fops::Directory`.
pub struct Directory {
    inner: Mutex<axfs::fops::Directory>,
    path: String,
}

impl Directory {
    pub fn new(inner: axfs::fops::Directory, path: String) -> Self {
        Self {
            inner: Mutex::new(inner),
            path,
        }
    }

    /// Get the path of the directory.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the inner node of the directory.
    pub fn inner(&self) -> MutexGuard<axfs::fops::Directory> {
        self.inner.lock()
    }
}

impl FileLike for Directory {
    fn read(&self, _buf: &mut [u8]) -> LinuxResult<usize> {
        Err(LinuxError::EBADF)
    }

    fn write(&self, _buf: &[u8]) -> LinuxResult<usize> {
        Err(LinuxError::EBADF)
    }

    fn stat(&self) -> LinuxResult<Kstat> {
        Ok(Kstat {
            mode: S_IFDIR | 0o755u32, // rwxr-xr-x
            ..Default::default()
        })
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn poll(&self) -> LinuxResult<PollState> {
        Ok(PollState {
            readable: true,
            writable: false,
        })
    }

    fn set_nonblocking(&self, _nonblocking: bool) -> LinuxResult {
        Ok(())
    }

    fn from_fd(fd: c_int) -> LinuxResult<Arc<Self>> {
        get_file_like(fd)?
            .into_any()
            .downcast::<Self>()
            .map_err(|_| LinuxError::ENOTDIR)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Kstat {
    pub ino: u64,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub size: u64,
    pub blocks: u64,
    pub blksize: u32,
}

impl Default for Kstat {
    fn default() -> Self {
        Self {
            ino: 1,
            nlink: 1,
            uid: 1,
            gid: 1,
            mode: 0,
            size: 0,
            blocks: 0,
            blksize: 4096,
        }
    }
}

impl From<Kstat> for stat {
    fn from(value: Kstat) -> Self {
        // SAFETY: valid for stat
        let mut stat: stat = unsafe { core::mem::zeroed() };
        stat.st_ino = value.ino as _;
        stat.st_nlink = value.nlink as _;
        stat.st_mode = value.mode as _;
        stat.st_uid = value.uid as _;
        stat.st_gid = value.gid as _;
        stat.st_size = value.size as _;
        stat.st_blksize = value.blksize as _;
        stat.st_blocks = value.blocks as _;

        stat
    }
}

impl From<Kstat> for statx {
    fn from(value: Kstat) -> Self {
        // SAFETY: valid for statx
        let mut statx: statx = unsafe { core::mem::zeroed() };
        statx.stx_blksize = value.blksize as _;
        statx.stx_attributes = value.mode as _;
        statx.stx_nlink = value.nlink as _;
        statx.stx_uid = value.uid as _;
        statx.stx_gid = value.gid as _;
        statx.stx_mode = value.mode as _;
        statx.stx_ino = value.ino as _;
        statx.stx_size = value.size as _;
        statx.stx_blocks = value.blocks as _;

        statx
    }
}
