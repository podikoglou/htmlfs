use anyhow::Context;
use dom_query::{Document, NodeId, NodeRef};
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use std::collections::HashMap;
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

    pub inode_to_id: HashMap<u64, NodeId>,
    pub id_to_inode: HashMap<NodeId, u64>,
}

impl HTMLFS {
    pub fn new(file: File) -> Self {
        Self {
            backend_file: file,
            document: Document::default(),
            inode_to_id: HashMap::default(),
            id_to_inode: HashMap::default(),
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
        self.refresh_inodes();

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

    pub fn refresh_inodes(&mut self) {
        // necessary?
        self.inode_to_id.clear();
        self.id_to_inode.clear();

        Self::refresh_inodes_rec(&mut self.inode_to_id, self.document.root(), 2);

        // does this suck?
        for (inode, id) in &self.inode_to_id {
            self.id_to_inode.insert(*id, *inode);
        }
    }

    fn refresh_inodes_rec(
        mapping: &mut HashMap<u64, NodeId>,
        node: NodeRef,
        mut counter: u64,
    ) -> u64 {
        mapping.insert(counter, node.id);
        counter += 1;

        for child in node.children() {
            counter = Self::refresh_inodes_rec(mapping, child, counter)
        }

        counter
    }

    pub fn mount(self, path: &String, options: Option<Vec<MountOption>>) -> anyhow::Result<()> {
        fuser::mount2(self, path, &options.unwrap_or_else(|| default_options()))
            .context("couldn't mount filesystem")
    }
}

impl<'a> Filesystem for HTMLFS {
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
        match self.inode_to_id.get(&ino) {
            Some(_) => reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]),
            None => reply.error(ENOENT),
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

        let root = self.document.root();

        let children = root.children();

        let entries = children.iter().enumerate().map(|(idx, child)| {
            let is_empty = child.children().is_empty();

            let name = child.element_ref().unwrap().name.local.to_string();

            (
                idx as u64,
                if is_empty {
                    FileType::RegularFile
                } else {
                    FileType::Directory
                },
                name,
            )
        });

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
