use anyhow::Context;
use dom_query::Document;
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::FileExt;
use std::time::{Duration, UNIX_EPOCH};

const TTL: Duration = Duration::from_secs(1);

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

const HELLO_TXT_CONTENT: &str = "Hello World!\n";

const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 13,
    blocks: 1,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

pub struct HTMLFS {
    pub backend_file: File,
    pub document: Document,
}

impl HTMLFS {
    pub fn new(file: File) -> Self {
        Self {
            backend_file: file,
            document: Document::default(),
        }
    }

    /// Updates the internal document with the contents of the backend file.
    pub fn download(&mut self) -> anyhow::Result<()> {
        // read file
        let mut content = String::default();
        self.backend_file.read_to_string(&mut content)?;

        // parse document
        let document = Document::from(content);

        // update internal document
        self.document = document;

        Ok(())
    }

    /// Updates the backend file with the internal document.
    pub fn upload(&mut self) -> anyhow::Result<usize> {
        let text = self.document.formatted_text().to_string();
        let bytes = text.as_bytes();

        self.backend_file
            .write_at(bytes, 0)
            .context("can't write to file")
    }

    pub fn mount(self, path: &String, options: Option<Vec<MountOption>>) -> anyhow::Result<()> {
        fuser::mount2(self, path, &options.unwrap_or_else(|| default_options()))
            .context("couldn't mount filesystem")
    }
}

impl Filesystem for HTMLFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == 1 && name.to_str() == Some("hello.txt") {
            reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        if ino == 2 {
            reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
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
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "hello.txt"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }
}

fn default_options() -> Vec<MountOption> {
    let mut options = vec![MountOption::RO, MountOption::FSName("htmlfs".to_string())];

    options.push(MountOption::AutoUnmount);
    options.push(MountOption::AllowRoot);

    options
}
