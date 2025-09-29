//! FUSE filesystem driver implementation

use cb_core::config::FuseConfig;
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen,
    Request,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

// Add platform-specific metadata support
#[cfg(target_os = "linux")]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[cfg(target_os = "macos")]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

// Helper function to convert file types
fn file_type_to_kind(file_type: std::fs::FileType) -> fuser::FileType {
    if file_type.is_dir() {
        fuser::FileType::Directory
    } else if file_type.is_file() {
        fuser::FileType::RegularFile
    } else if file_type.is_symlink() {
        fuser::FileType::Symlink
    } else {
        // Default for other types like block device, char device, fifo, socket
        fuser::FileType::RegularFile
    }
}

// Constants for error handling
const ENOENT: i32 = libc::ENOENT;
#[allow(dead_code)]
const EIO: i32 = libc::EIO;
const SYSTEM_TIME_UNIX_EPOCH: SystemTime = SystemTime::UNIX_EPOCH;

const TTL: Duration = Duration::from_secs(1);

/// FUSE filesystem implementation for Codebuddy
pub struct CodeflowFS {
    /// Path to the real workspace on disk
    #[allow(dead_code)]
    workspace_path: PathBuf,
    /// Cache of file attributes to avoid repeated filesystem calls
    #[allow(dead_code)]
    attr_cache: HashMap<u64, FileAttr>,
    /// Next available inode number
    next_inode: u64,
    /// Mapping from inode to real path
    inode_to_path: HashMap<u64, PathBuf>,
    /// Mapping from path to inode
    path_to_inode: HashMap<PathBuf, u64>,
}

impl CodeflowFS {
    /// Create a new CodeflowFS instance
    pub fn new(workspace_path: PathBuf) -> Self {
        let mut fs = Self {
            workspace_path: workspace_path.clone(),
            attr_cache: HashMap::new(),
            next_inode: 2, // Start at 2, as 1 is reserved for root
            inode_to_path: HashMap::new(),
            path_to_inode: HashMap::new(),
        };

        // Initialize root directory
        fs.inode_to_path.insert(1, workspace_path.clone());
        fs.path_to_inode.insert(workspace_path, 1);

        fs
    }

    /// Get or assign an inode for a given path
    #[allow(dead_code)]
    fn get_or_assign_inode(&mut self, path: &Path) -> u64 {
        if let Some(&inode) = self.path_to_inode.get(path) {
            return inode;
        }

        let inode = self.next_inode;
        self.next_inode += 1;
        self.inode_to_path.insert(inode, path.to_path_buf());
        self.path_to_inode.insert(path.to_path_buf(), inode);
        inode
    }

    /// Convert a filesystem metadata to FUSE FileAttr
    #[allow(dead_code)]
    fn metadata_to_attr(&self, metadata: &fs::Metadata, ino: u64) -> FileAttr {
        let file_type = if metadata.is_dir() {
            FileType::Directory
        } else if metadata.is_file() {
            FileType::RegularFile
        } else if metadata.file_type().is_symlink() {
            FileType::Symlink
        } else {
            FileType::RegularFile // Default fallback
        };

        let atime = metadata
            .accessed()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));

        let mtime = metadata
            .modified()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));

        let ctime = metadata
            .created()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));

        FileAttr {
            ino,
            size: metadata.len(),
            blocks: metadata.len().div_ceil(512), // 512-byte blocks
            atime: SystemTime::UNIX_EPOCH + atime,
            mtime: SystemTime::UNIX_EPOCH + mtime,
            ctime: SystemTime::UNIX_EPOCH + ctime,
            crtime: SystemTime::UNIX_EPOCH + ctime,
            kind: file_type,
            perm: if metadata.is_dir() { 0o755 } else { 0o644 },
            nlink: 1,
            uid: 1000, // Default user ID
            gid: 1000, // Default group ID
            rdev: 0,
            flags: 0,
            blksize: 4096,
        }
    }

    /// Resolve a path relative to the workspace
    #[allow(dead_code)]
    fn resolve_path(&self, parent_ino: u64, name: &OsStr) -> Option<PathBuf> {
        let parent_path = self.inode_to_path.get(&parent_ino)?;
        Some(parent_path.join(name))
    }
}

impl Filesystem for CodeflowFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let parent_path = self.inode_to_path.get(&parent).unwrap().clone();
        let path = parent_path.join(name);
        if !path.exists() {
            reply.error(ENOENT);
            return;
        }

        let inode = match self.path_to_inode.get(&path) {
            Some(ino) => *ino,
            None => {
                let ino = self.next_inode;
                self.next_inode += 1;
                self.inode_to_path.insert(ino, path.clone());
                self.path_to_inode.insert(path.clone(), ino);
                ino
            }
        };

        match std::fs::metadata(&path) {
            Ok(metadata) => {
                let attrs = fuser::FileAttr {
                    ino: inode,
                    size: metadata.len(),
                    blocks: metadata.blocks(),
                    atime: metadata.accessed().unwrap_or(SYSTEM_TIME_UNIX_EPOCH),
                    mtime: metadata.modified().unwrap_or(SYSTEM_TIME_UNIX_EPOCH),
                    ctime: metadata.created().unwrap_or(SYSTEM_TIME_UNIX_EPOCH),
                    crtime: metadata.created().unwrap_or(SYSTEM_TIME_UNIX_EPOCH),
                    kind: file_type_to_kind(metadata.file_type()),
                    perm: metadata.permissions().mode() as u16,
                    nlink: metadata.nlink() as u32,
                    uid: metadata.uid(),
                    gid: metadata.gid(),
                    rdev: metadata.rdev() as u32,
                    blksize: metadata.blksize() as u32,
                    flags: 0,
                };
                reply.entry(&TTL, &attrs, 0);
            }
            Err(e) => reply.error(e.raw_os_error().unwrap_or(ENOENT)),
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        if let Some(path) = self.inode_to_path.get(&ino) {
            match std::fs::metadata(path) {
                Ok(metadata) => {
                    let attrs = fuser::FileAttr {
                        ino,
                        size: metadata.len(),
                        blocks: metadata.blocks(),
                        atime: metadata.accessed().unwrap_or(SYSTEM_TIME_UNIX_EPOCH),
                        mtime: metadata.modified().unwrap_or(SYSTEM_TIME_UNIX_EPOCH),
                        ctime: metadata.created().unwrap_or(SYSTEM_TIME_UNIX_EPOCH),
                        crtime: metadata.created().unwrap_or(SYSTEM_TIME_UNIX_EPOCH),
                        kind: file_type_to_kind(metadata.file_type()),
                        perm: metadata.permissions().mode() as u16,
                        nlink: metadata.nlink() as u32,
                        uid: metadata.uid(),
                        gid: metadata.gid(),
                        rdev: metadata.rdev() as u32,
                        blksize: metadata.blksize() as u32,
                        flags: 0,
                    };
                    reply.attr(&TTL, &attrs);
                }
                Err(e) => reply.error(e.raw_os_error().unwrap_or(ENOENT)),
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if let Some(path) = self.inode_to_path.get(&ino) {
            if let Ok(read_dir) = std::fs::read_dir(path) {
                let entries = read_dir
                    .filter_map(|res| res.ok())
                    .skip(offset as usize)
                    .collect::<Vec<_>>();

                for (i, entry) in entries.iter().enumerate() {
                    let path = entry.path();
                    let file_name = path.file_name().unwrap_or_default();
                    let file_type = entry.file_type().unwrap();

                    let inode = match self.path_to_inode.get(&path) {
                        Some(ino) => *ino,
                        None => {
                            let new_ino = self.next_inode;
                            self.next_inode += 1;
                            self.inode_to_path.insert(new_ino, path.clone());
                            self.path_to_inode.insert(path.clone(), new_ino);
                            new_ino
                        }
                    };

                    if reply.add(
                        inode,
                        offset + i as i64 + 1,
                        file_type_to_kind(file_type),
                        file_name,
                    ) {
                        break;
                    }
                }
                reply.ok();
            } else {
                reply.error(ENOENT);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn open(&mut self, _req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        debug!(ino = %ino, "open");

        let path = match self.inode_to_path.get(&ino) {
            Some(path) => path,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // For simplicity, just check if the file exists
        if path.exists() && path.is_file() {
            reply.opened(0, 0); // fh=0, flags=0
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        debug!(ino = %ino, offset = %offset, size = %size, "read");

        let path = match self.inode_to_path.get(&ino) {
            Some(path) => path,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        match fs::read(path) {
            Ok(data) => {
                let start = offset as usize;
                let end = std::cmp::min(start + size as usize, data.len());

                if start >= data.len() {
                    reply.data(&[]);
                } else {
                    reply.data(&data[start..end]);
                }
            }
            Err(err) => {
                warn!(path = ?path, error = %err, "Failed to read file");
                reply.error(libc::EIO);
            }
        }
    }
}

/// Start the FUSE mount in a background thread
pub fn start_fuse_mount(
    config: &FuseConfig,
    workspace_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mount_point = config.mount_point.clone();
    let workspace_path = workspace_path.to_path_buf();

    info!(
        "Starting FUSE mount: {} -> {}",
        workspace_path.display(),
        mount_point.display()
    );

    // Validate mount point exists
    if !mount_point.exists() {
        return Err(format!("Mount point {:?} does not exist", mount_point).into());
    }

    if !mount_point.is_dir() {
        return Err(format!("Mount point {:?} is not a directory", mount_point).into());
    }

    // Validate workspace path exists
    if !workspace_path.exists() {
        return Err(format!("Workspace path {:?} does not exist", workspace_path).into());
    }

    if !workspace_path.is_dir() {
        return Err(format!("Workspace path {:?} is not a directory", workspace_path).into());
    }

    let filesystem = CodeflowFS::new(workspace_path.clone());

    // Spawn FUSE mount in a background thread
    let mount_point_clone = mount_point.clone();
    std::thread::spawn(move || {
        info!(mount_point = ?mount_point_clone, "Mounting FUSE filesystem");

        let options = vec![
            fuser::MountOption::RO, // Read-only mount
            fuser::MountOption::FSName("codebuddy".to_string()),
            fuser::MountOption::AutoUnmount,
        ];

        if let Err(err) = fuser::mount2(filesystem, &mount_point_clone, &options) {
            error!(error = %err, "FUSE mount failed");
        } else {
            info!("FUSE filesystem unmounted");
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_codeflow_fs_creation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();

        let fs = CodeflowFS::new(workspace_path.clone());

        assert_eq!(fs.workspace_path, workspace_path);
        assert_eq!(fs.next_inode, 2);
        assert!(fs.inode_to_path.contains_key(&1));
        assert!(fs.path_to_inode.contains_key(&workspace_path));
    }

    #[test]
    fn test_get_or_assign_inode() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let mut fs = CodeflowFS::new(workspace_path);

        let test_path = Path::new("/test/path");
        let inode1 = fs.get_or_assign_inode(test_path);
        let inode2 = fs.get_or_assign_inode(test_path);

        assert_eq!(inode1, inode2); // Should return same inode for same path
        assert_eq!(inode1, 2); // First assigned inode should be 2
    }

    #[test]
    fn test_metadata_to_attr() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let fs = CodeflowFS::new(workspace_path.clone());

        let metadata = fs::metadata(&workspace_path).unwrap();
        let attr = fs.metadata_to_attr(&metadata, 1);

        assert_eq!(attr.ino, 1);
        assert_eq!(attr.kind, FileType::Directory);
        assert_eq!(attr.perm, 0o755);
    }
}
