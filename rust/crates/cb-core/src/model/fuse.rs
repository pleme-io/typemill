//! FUSE filesystem configuration and types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// FUSE filesystem configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FuseConfig {
    /// Mount point for the FUSE filesystem
    pub mount_point: PathBuf,
    /// Enable read-only mode
    pub read_only: bool,
    /// Cache timeout in seconds
    pub cache_timeout_seconds: u64,
    /// Maximum file size to cache in bytes
    pub max_file_size_bytes: u64,
    /// Enable debug logging for FUSE operations
    #[serde(default)]
    pub debug: bool,
    /// Additional mount options
    #[serde(default)]
    pub mount_options: Vec<String>,
}

/// FUSE operation result
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FuseResult<T> {
    Ok(T),
    Err(FuseError),
}

/// FUSE operation errors
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FuseError {
    /// File or directory not found
    NotFound,
    /// Permission denied
    PermissionDenied,
    /// Invalid argument
    InvalidArgument,
    /// I/O error
    IoError(String),
    /// Not a directory
    NotDirectory,
    /// Is a directory
    IsDirectory,
    /// Directory not empty
    DirectoryNotEmpty,
    /// File exists
    FileExists,
    /// No space left on device
    NoSpaceLeft,
    /// Read-only filesystem
    ReadOnlyFilesystem,
    /// Operation not supported
    NotSupported,
    /// Internal error
    Internal(String),
}

/// File attributes for FUSE operations
#[derive(Debug, Clone, PartialEq)]
pub struct FuseFileAttr {
    /// File size in bytes
    pub size: u64,
    /// Number of blocks
    pub blocks: u64,
    /// Time of last access
    pub atime: std::time::SystemTime,
    /// Time of last modification
    pub mtime: std::time::SystemTime,
    /// Time of last metadata change
    pub ctime: std::time::SystemTime,
    /// Time of creation (macOS only)
    pub crtime: std::time::SystemTime,
    /// File type and permissions
    pub perm: u16,
    /// Number of hard links
    pub nlink: u32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Device ID (if special file)
    pub rdev: u32,
    /// Filesystem block size
    pub blksize: u32,
    /// Flags (macOS only)
    pub flags: u32,
}

/// Directory entry for FUSE operations
#[derive(Debug, Clone, PartialEq)]
pub struct FuseDirEntry {
    /// File name
    pub name: String,
    /// File attributes
    pub attr: FuseFileAttr,
    /// Entry offset
    pub offset: i64,
}

/// FUSE filesystem statistics
#[derive(Debug, Clone, PartialEq)]
pub struct FuseStatfs {
    /// Total blocks in filesystem
    pub blocks: u64,
    /// Free blocks in filesystem
    pub bfree: u64,
    /// Free blocks available to unprivileged user
    pub bavail: u64,
    /// Total file nodes in filesystem
    pub files: u64,
    /// Free file nodes in filesystem
    pub ffree: u64,
    /// Filesystem block size
    pub bsize: u32,
    /// Maximum filename length
    pub namelen: u32,
    /// Fragment size
    pub frsize: u32,
}

impl Default for FuseConfig {
    fn default() -> Self {
        Self {
            mount_point: PathBuf::from("/tmp/codebuddy"),
            read_only: true,
            cache_timeout_seconds: 60,
            max_file_size_bytes: 10 * 1024 * 1024, // 10 MB
            debug: false,
            mount_options: vec![
                "auto_unmount".to_string(),
                "default_permissions".to_string(),
            ],
        }
    }
}

impl<T> FuseResult<T> {
    /// Check if the result is ok
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    /// Check if the result is an error
    pub fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    /// Convert to a standard Result
    pub fn into_result(self) -> Result<T, FuseError> {
        match self {
            Self::Ok(value) => Ok(value),
            Self::Err(error) => Err(error),
        }
    }
}

impl<T> From<Result<T, FuseError>> for FuseResult<T> {
    fn from(result: Result<T, FuseError>) -> Self {
        match result {
            Ok(value) => Self::Ok(value),
            Err(error) => Self::Err(error),
        }
    }
}

impl std::fmt::Display for FuseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "File or directory not found"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::InvalidArgument => write!(f, "Invalid argument"),
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
            Self::NotDirectory => write!(f, "Not a directory"),
            Self::IsDirectory => write!(f, "Is a directory"),
            Self::DirectoryNotEmpty => write!(f, "Directory not empty"),
            Self::FileExists => write!(f, "File exists"),
            Self::NoSpaceLeft => write!(f, "No space left on device"),
            Self::ReadOnlyFilesystem => write!(f, "Read-only filesystem"),
            Self::NotSupported => write!(f, "Operation not supported"),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for FuseError {}

impl Default for FuseFileAttr {
    fn default() -> Self {
        let now = std::time::SystemTime::now();
        Self {
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            perm: 0o644,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }
}
