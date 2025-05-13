use core::{ffi::c_int, fmt, ops::Deref};

use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
};
use axerrno::{AxError, AxResult, LinuxError, LinuxResult};
use axfs::api::canonicalize;
use linux_raw_sys::general::AT_FDCWD;
use spin::RwLock;

use crate::file::{Directory, File, FileLike};

/// A normalized file path representation
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct FilePath(String);

impl FilePath {
    /// Create a new `FilePath` from a path string, the path will be normalized.
    /// The input path can be an absolute path or a relative path.
    pub fn new<P: AsRef<str>>(path: P) -> AxResult<Self> {
        let path = path.as_ref();
        let canonical = canonicalize(path).map_err(|_| AxError::NotFound)?;
        let mut new_path = canonical.trim().to_string();

        // If the original path ends with '/', then the normalized path should also end with '/'
        if path.ends_with('/') && !new_path.ends_with('/') {
            new_path.push('/');
        }

        assert!(
            new_path.starts_with('/'),
            "canonical path should start with /"
        );

        Ok(Self(HARDLINK_MANAGER.real_path(&new_path)))
    }

    /// Returns a string slice of the underlying path
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the parent directory path
    pub fn parent(&self) -> AxResult<&str> {
        if self.is_root() {
            return Ok("/");
        }

        // Find the last slash, considering possible trailing slash
        let mut path = self.as_str();
        if path.ends_with('/') {
            path = path.strip_suffix('/').unwrap();
        }
        let pos = path.rfind('/').ok_or(AxError::NotFound)?;

        Ok(&path[..=pos])
    }

    /// Returns the file or directory name component
    pub fn name(&self) -> AxResult<&str> {
        if self.is_root() {
            return Ok("/");
        }

        let mut path = self.as_str();
        if path.ends_with('/') {
            path = path.strip_suffix('/').unwrap();
        }
        let start_pos = path.rfind('/').ok_or(AxError::NotFound)?;

        let end_pos = if path.ends_with('/') {
            path.len() - 1
        } else {
            path.len()
        };
        Ok(&path[start_pos + 1..end_pos])
    }

    /// Check if it's the root directory
    pub fn is_root(&self) -> bool {
        self.0 == "/"
    }

    /// Check if it's a directory (ends with '/')
    pub fn is_dir(&self) -> bool {
        self.0.ends_with('/')
    }

    /// Check if it's a regular file (doesn't end with '/')
    pub fn is_file(&self) -> bool {
        !self.is_dir()
    }

    /// Whether the path exists
    pub fn exists(&self) -> bool {
        axfs::api::absolute_path_exists(&self.0)
    }

    /// Check if this path starts with the given prefix path
    pub fn starts_with(&self, prefix: &FilePath) -> bool {
        self.0.starts_with(&prefix.0)
    }

    /// Check if this path ends with the given suffix path
    pub fn ends_with(&self, suffix: &FilePath) -> bool {
        self.0.ends_with(&suffix.0)
    }

    /// Join this path with a relative path component
    pub fn join<P: AsRef<str>>(&self, path: P) -> AxResult<Self> {
        let mut new_path = self.0.clone();
        if !new_path.ends_with('/') {
            new_path.push('/');
        }
        new_path.push_str(path.as_ref());
        FilePath::new(new_path)
    }

    /// Returns an iterator of the path components
    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0.trim_matches('/').split('/')
    }
}

impl fmt::Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for FilePath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FilePath {
    fn from(s: &str) -> Self {
        FilePath::new(s).unwrap()
    }
}

impl Deref for FilePath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Error types
#[derive(Debug)]
pub enum LinkError {
    LinkExists,  // Link already exists
    InvalidPath, // Invalid path
    NotFound,    // File not found
    NotFile,     // Not a file
}

impl From<LinkError> for AxError {
    fn from(err: LinkError) -> AxError {
        match err {
            LinkError::LinkExists => AxError::AlreadyExists,
            LinkError::InvalidPath => AxError::InvalidInput,
            LinkError::NotFound => AxError::NotFound,
            LinkError::NotFile => AxError::InvalidInput,
        }
    }
}

impl From<LinkError> for LinuxError {
    fn from(err: LinkError) -> LinuxError {
        AxError::from(err).into()
    }
}

/// A global hardlink manager
pub static HARDLINK_MANAGER: HardlinkManager = HardlinkManager::new();

/// A manager for hardlinks
pub struct HardlinkManager {
    inner: RwLock<LinkManagerInner>,
}
struct LinkManagerInner {
    links: BTreeMap<String, String>,
    ref_counts: BTreeMap<String, usize>,
}

// All operations on inner are in atomic_prefixed functions
impl HardlinkManager {
    const fn new() -> Self {
        Self {
            inner: RwLock::new(LinkManagerInner {
                links: BTreeMap::new(),
                ref_counts: BTreeMap::new(),
            }),
        }
    }

    /// Create a link
    /// Returns `LinkError::NotFound` if the target path doesn't exist
    /// Returns `LinkError::NotFile` if the target path is not a file
    pub fn create_link(&self, src: &FilePath, dst: &FilePath) -> Result<(), LinkError> {
        if !dst.exists() {
            return Err(LinkError::NotFound);
        }
        if !dst.is_dir() {
            return Err(LinkError::NotFile);
        }

        let mut inner = self.inner.write();
        self.atomic_link_update(&mut inner, src, dst);
        Ok(())
    }

    /// Remove a link
    /// Delete the file when link count is zero or no links exist
    /// Returns `None` if the path has no link or the file doesn't exist
    /// Otherwise returns the target path of the link
    pub fn remove_link(&self, src: &FilePath) -> Option<String> {
        let mut inner = self.inner.write();
        self.atomic_link_remove(&mut inner, src).or_else(|| {
            axfs::api::remove_file(src.as_str())
                .ok()
                .map(|_| src.to_string())
        })
    }

    pub fn real_path(&self, path: &str) -> String {
        self.inner
            .read()
            .links
            .get(path)
            .cloned()
            .unwrap_or_else(|| path.to_string())
    }

    pub fn link_count(&self, path: &FilePath) -> usize {
        let inner = self.inner.read();
        inner
            .ref_counts
            .get(path.as_str())
            .copied()
            .unwrap_or_else(|| if path.exists() { 1 } else { 0 })
    }

    // Atomic operation helpers

    /// Create or update a link
    /// If the link already exists, update the target path
    /// Returns `LinkError::NotFound` if the target path doesn't exist
    fn atomic_link_update(&self, inner: &mut LinkManagerInner, src: &FilePath, dst: &FilePath) {
        if let Some(old_dst) = inner.links.get(src.as_str()) {
            if old_dst == dst.as_str() {
                return;
            }
            self.decrease_ref_count(inner, &old_dst.to_string());
        }
        inner.links.insert(src.to_string(), dst.to_string());
        *inner.ref_counts.entry(dst.to_string()).or_insert(0) += 1;
    }

    /// Remove a link
    /// Returns `None` if the link doesn't exist, otherwise returns the target path of the link
    fn atomic_link_remove(&self, inner: &mut LinkManagerInner, src: &FilePath) -> Option<String> {
        inner.links.remove(src.as_str()).inspect(|dst| {
            self.decrease_ref_count(inner, dst);
        })
    }

    /// Decrease reference count
    /// If reference count is zero, delete the link and file. Returns `None` if deleting file fails
    /// Returns `None` if the link doesn't exist
    fn decrease_ref_count(&self, inner: &mut LinkManagerInner, path: &str) -> Option<()> {
        match inner.ref_counts.get_mut(path) {
            Some(count) => {
                *count -= 1;
                if *count == 0 {
                    inner.ref_counts.remove(path);
                    axfs::api::remove_file(path).ok()?
                }
                Some(())
            }
            None => {
                axlog::error!("link exists but ref count is zero");
                None
            }
        }
    }
}

pub fn handle_file_path(dirfd: c_int, path: &str) -> LinuxResult<FilePath> {
    if path.starts_with('/') {
        Ok(FilePath::new(path)?)
    } else if path.is_empty() {
        Ok(FilePath::new(File::from_fd(dirfd)?.path())?)
    } else {
        let base = if dirfd == AT_FDCWD {
            FilePath::new("")?
        } else {
            FilePath::new(Directory::from_fd(dirfd)?.path())?
        };
        Ok(base.join(path)?)
    }
}
