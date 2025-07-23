use std::fs::File;

use anyhow::Context;
use clap::{Arg, Command};

use crate::fs::HTMLFS;

pub mod fs;

fn main() -> anyhow::Result<()> {
    let matches = Command::new("htmlfs")
        .author("alex")
        .arg(
            Arg::new("FILE")
                .required(true)
                .index(1)
                .help("Path to the backend HTML file"),
        )
        .arg(
            Arg::new("MOUNT_POINT")
                .required(true)
                .index(2)
                .help("Path to mount filesystem to"),
        )
        .get_matches();

    let backend = matches.get_one::<String>("FILE").unwrap();
    let mountpoint = matches.get_one::<String>("MOUNT_POINT").unwrap();

    let file = File::options()
        .read(true)
        .write(true)
        .open(backend)
        .context("couldn't open backend file")?;

    let mut fs = HTMLFS::new(file);
    fs.download()?;

    fs.mount(mountpoint, None)
}
