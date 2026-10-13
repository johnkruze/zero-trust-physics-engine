//! Opening a file that holds a stored record: the head, the rows and the
//! frames of a dataset.
//!
//! A stored file must be a regular file, reached without following a symbolic
//! link. A link, a named pipe, a device, a socket or a folder is refused by
//! name before one byte is read. The size is taken from the open file, and
//! the file is read for no more than that many bytes, so a file that grows
//! after it was opened is not read past the size taken. This is the one rule
//! the gateway, the `genesis_seal` program and the Parquet tool all use.

use std::fs::{File, Metadata};
use std::io::{self, Read, Take};
use std::path::{Path, PathBuf};

/// Why a stored file or folder was not opened.
#[derive(Debug)]
pub enum StoredFileRefusal {
    /// The path is a symbolic link. It is never followed.
    Link(PathBuf),
    /// The path is not a regular file (or, for a folder, not a folder). The
    /// word says what it is.
    NotRegular(PathBuf, &'static str),
    /// The system could not open it.
    NotRead(PathBuf, io::Error),
}

impl StoredFileRefusal {
    /// The same sentence without the path, for an answer that must not name
    /// where the file is kept: "is a symbolic link", "is a named pipe",
    /// or the system's own reason.
    pub fn what(&self) -> String {
        match self {
            StoredFileRefusal::Link(_) => "is a symbolic link".to_string(),
            StoredFileRefusal::NotRegular(_, what) => format!("is {what}"),
            StoredFileRefusal::NotRead(_, error) => format!("could not be read: {error}"),
        }
    }

    /// True when the path is not there.
    pub fn is_missing(&self) -> bool {
        matches!(self, StoredFileRefusal::NotRead(_, error) if error.kind() == io::ErrorKind::NotFound)
    }
}

impl std::fmt::Display for StoredFileRefusal {
    fn fmt(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StoredFileRefusal::Link(path) => write!(
                out,
                "'{}' is a symbolic link. A stored file must be a regular file reached without following a link.",
                path.display()
            ),
            StoredFileRefusal::NotRegular(path, what) => write!(
                out,
                "'{}' is {what}. A stored file must be a regular file.",
                path.display()
            ),
            StoredFileRefusal::NotRead(path, error) => {
                write!(out, "'{}' could not be read: {error}", path.display())
            }
        }
    }
}

impl std::error::Error for StoredFileRefusal {}

fn kind_of(metadata: &Metadata) -> &'static str {
    use std::os::unix::fs::FileTypeExt;
    let kind = metadata.file_type();
    if kind.is_dir() {
        "a folder"
    } else if kind.is_fifo() {
        "a named pipe"
    } else if kind.is_char_device() || kind.is_block_device() {
        "a device"
    } else if kind.is_socket() {
        "a socket"
    } else if kind.is_symlink() {
        "a symbolic link"
    } else {
        "not a regular file"
    }
}

/// A folder that is a folder itself and not a link to one.
pub fn require_plain_folder(path: &Path) -> Result<(), StoredFileRefusal> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| StoredFileRefusal::NotRead(path.to_path_buf(), error))?;
    if metadata.file_type().is_symlink() {
        return Err(StoredFileRefusal::Link(path.to_path_buf()));
    }
    if !metadata.is_dir() {
        return Err(StoredFileRefusal::NotRegular(path.to_path_buf(), kind_of(&metadata)));
    }
    Ok(())
}

/// A regular file, open for reading, and the size it had when it was opened.
#[derive(Debug)]
pub struct RegularFile {
    file: File,
    size: u64,
}

impl RegularFile {
    /// The size, in bytes, taken from the open file.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// The file as a reader that ends after `size()` bytes.
    pub fn reader(self) -> Take<File> {
        self.file.take(self.size)
    }

    /// SHA-256 of the file, in lowercase hex, read as a stream for no more
    /// than the size taken at open.
    pub fn sha256_hex(self) -> io::Result<String> {
        use sha2::{Digest, Sha256};
        let mut reader = self.reader();
        let mut hasher = Sha256::new();
        let mut block = vec![0_u8; 1 << 20];
        loop {
            match reader.read(&mut block) {
                Ok(0) => break,
                Ok(count) => hasher.update(&block[..count]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(hex::encode(hasher.finalize()))
    }

    /// Every byte of the file, up to the size taken at open.
    pub fn read_all(self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(self.size.min(1 << 30) as usize);
        self.reader().read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

/// Open `path` for reading if it is a regular file and not a link. The open
/// never blocks, so a named pipe is refused instead of waited for.
pub fn open_regular(path: &Path) -> Result<RegularFile, StoredFileRefusal> {
    use std::os::unix::fs::OpenOptionsExt;
    let before = std::fs::symlink_metadata(path)
        .map_err(|error| StoredFileRefusal::NotRead(path.to_path_buf(), error))?;
    if before.file_type().is_symlink() {
        return Err(StoredFileRefusal::Link(path.to_path_buf()));
    }
    if !before.is_file() {
        return Err(StoredFileRefusal::NotRegular(path.to_path_buf(), kind_of(&before)));
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|error| {
            if error.raw_os_error() == Some(libc::ELOOP) {
                StoredFileRefusal::Link(path.to_path_buf())
            } else {
                StoredFileRefusal::NotRead(path.to_path_buf(), error)
            }
        })?;
    // The file opened may not be the one looked at: ask the open file.
    let after = file
        .metadata()
        .map_err(|error| StoredFileRefusal::NotRead(path.to_path_buf(), error))?;
    if !after.is_file() {
        return Err(StoredFileRefusal::NotRegular(path.to_path_buf(), kind_of(&after)));
    }
    Ok(RegularFile {
        file,
        size: after.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::symlink;

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("stored_file_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn make_fifo(path: &Path) {
        let text = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(text.as_ptr(), 0o600) }, 0);
    }

    #[test]
    fn a_regular_file_opens_and_reads_its_size_and_no_more() {
        let folder = scratch("regular");
        let path = folder.join("a");
        std::fs::File::create(&path).unwrap().write_all(b"12345").unwrap();
        let opened = open_regular(&path).unwrap();
        assert_eq!(opened.size(), 5);
        // The file grows after it was opened: the bytes read are still five.
        std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"678").unwrap();
        assert_eq!(opened.read_all().unwrap(), b"12345");
    }

    #[test]
    fn a_link_a_pipe_a_device_and_a_folder_are_refused_by_name_without_waiting() {
        let folder = scratch("refused");
        let real = folder.join("real");
        std::fs::write(&real, b"x").unwrap();
        symlink(&real, folder.join("link")).unwrap();
        symlink("/dev/zero", folder.join("zero")).unwrap();
        make_fifo(&folder.join("pipe"));
        let words = |name: &str| open_regular(&folder.join(name)).unwrap_err().to_string();
        assert!(words("link").contains("symbolic link"), "{}", words("link"));
        assert!(words("zero").contains("symbolic link"));
        assert!(words("pipe").contains("a named pipe"), "{}", words("pipe"));
        assert!(open_regular(Path::new("/dev/zero")).unwrap_err().to_string().contains("a device"));
        assert!(words("").contains("a folder"));
        assert!(open_regular(&folder.join("absent")).unwrap_err().is_missing());
        assert!(require_plain_folder(&folder).is_ok());
        assert!(require_plain_folder(&real).is_err());
        symlink(&folder, folder.join("folderlink")).unwrap();
        assert!(matches!(
            require_plain_folder(&folder.join("folderlink")).unwrap_err(),
            StoredFileRefusal::Link(_)
        ));
    }
}
