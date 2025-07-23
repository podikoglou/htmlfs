use clap::{Arg, Command};

use crate::fs::HTMLFS;

pub mod fs;

fn main() -> anyhow::Result<()> {
    let matches = Command::new("htmlfs")
        .author("alex")
        .arg(
            Arg::new("MOUNT_POINT")
                .required(true)
                .index(1)
                .help("Path to mount filesystem to"),
        )
        .get_matches();

    let mountpoint = matches.get_one::<String>("MOUNT_POINT").unwrap();

    let fs = HTMLFS::new(None);

    fs.mount(mountpoint, None)
}
