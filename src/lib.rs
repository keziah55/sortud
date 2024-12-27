// print results in nice table
// use different colours for files and dirs

// future features:
// - follow or ignore symlinks
// - exclude patterns
// - sort by size, modified date or name (option for dirs first)

use chrono::{DateTime, Utc};
use clap::Parser;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time;
use std::vec;

#[derive(Parser)]
#[command(version = "0.1")]
#[command(about="display sizes of files and directories", long_about=None)]
pub struct Cli {
    /// only descend up to N levels below the current directory
    #[arg(long, short = 'd', value_name = "N")]
    max_depth: Option<u8>,

    /// print results in ascending order, rather than the default descending
    #[arg(short, long)]
    ascending: bool,

    /// print sizes in human readable format (e.g. 1K, 23M, 4G)
    #[arg(short = 's', long)]
    humanize: bool,

    /// like humanize, but use powers of 1000 instead of 1024
    #[arg(long)]
    si: bool,

    /// show time of last modification of file, or any file in sub-directory
    #[arg(short = 't', long)]
    time: bool,

    /// skip symlinks
    #[arg(long)]
    skip_symlinks: bool,

    /// file or path
    file: String,
}

pub enum ByteType {
    Binary,
    Decimal,
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum ItemType {
    File,
    Dir,
    Symlink,
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
enum ItemTypeColours {
    File = 7,
    FileHidden = 244,
    Dir = 39,
    DirHidden = 74,
    Symlink = 10,
    SymlinkHidden = 70,
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct FileInfo {
    pub path: PathBuf,
    pub depth: u8,
    pub file_type: ItemType,
    pub size: u64,
    pub modified: time::SystemTime,
    pub children: Option<Vec<FileInfo>>,
    pub accessible: bool,
}

impl FileInfo {
    pub fn to_string(&self, humanize: bool, byte_type: &ByteType, show_ts: bool) -> String {
        let space = " ".repeat(2);

        let mut size = if humanize {
            format_size(self.size, byte_type)
        } else {
            format!("{:>width$}", self.size, width = 7)
        };
        size = format!("{}{}", size, space);

        let ts = if show_ts {
            let date_time: DateTime<Utc> = self.modified.clone().into();
            format!("{}{}", date_time.format("%Y %b %d %H:%M:%S"), space)
        } else {
            String::from("")
        };

        // let s = format!("{}{}{}", size, ts, self.path.to_str().unwrap());
        let mut s = format!("{}{}{}", size, ts, self.pretty_path());

        let hidden = self.is_hidden();

        let colour = match self.file_type {
            ItemType::Dir => {
                if hidden {
                    ItemTypeColours::DirHidden
                } else {
                    ItemTypeColours::Dir
                }
            }
            ItemType::Symlink => {
                if hidden {
                    ItemTypeColours::SymlinkHidden
                } else {
                    ItemTypeColours::Symlink
                }
            }
            ItemType::File => {
                if hidden {
                    ItemTypeColours::FileHidden
                } else {
                    ItemTypeColours::File
                }
            }
        };

        s = format!("\x1b[38;5;{}m{:#}\x1b[0m", colour as i32, s);
        if !self.accessible {
            s = format!("{}\n  \x1b[38;5;3mCould not access contents\x1b[0m",s)
        }
        s
    }

    fn is_hidden(&self) -> bool {
        self.path
            .components()
            .last()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap()
            .starts_with(".")
    }

    fn first_component_is_pwd(&self) -> bool {
        self.path
            .components()
            .next()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap()
            == "."
    }

    fn pretty_path(&self) -> &str {
        if self.first_component_is_pwd() {
            let mut components = self.path.components();
            components.next(); // remove the first component
            components.as_path().to_str().unwrap()
        } else {
            self.path.to_str().unwrap()
        }
    }
}

impl fmt::Display for FileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.to_string(true, &ByteType::Binary, true);
        write!(f, "{}", s)
    }
}

fn format_size(size: u64, byte_type: &ByteType) -> String {
    let mut size_f = size as f64;
    let mut prefixes = vec![" ", "K", "M", "G", "T"];

    let div = match &byte_type {
        ByteType::Decimal => {
            prefixes[1] = "k";
            1000.0
        }
        ByteType::Binary => 1024.0,
    };

    let mut idx = 0;
    while size_f > div {
        size_f /= div;
        idx += 1;
    }

    format!("{0:7.3} {1}B", size_f, prefixes[idx])
}

fn print_results(
    path_info: &Vec<FileInfo>,
    humanize: bool,
    si: bool,
    show_ts: bool,
    max_depth: Option<u8>,
) {
    let byte_type = if si {
        ByteType::Decimal
    } else {
        ByteType::Binary
    };

    for info in path_info {
        if let Some(d) = max_depth {
            if d < info.depth {
                return;
            }
        }
        let s = info.to_string(humanize, &byte_type, show_ts);
        println!("{}", s);
        if let Some(v) = &info.children {
            print_results(v, humanize, si, show_ts, max_depth)
        }
    }
}

fn get_file_type(path: &Path) -> ItemType {
    let md = fs::symlink_metadata(path).unwrap();
    if md.is_file() {
        ItemType::File
    } else if md.is_dir() {
        ItemType::Dir
    } else if md.is_symlink() {
        ItemType::Symlink
    } else {
        panic!("Could not identify type")
    }
}

fn get_file_info(
    path: &Path,
    depth: u8,
    md: &fs::Metadata,
    parent_is_symlink: bool,
) -> Result<FileInfo, Box<dyn Error>> {
    let modified = md.modified()?;
    // make new PathBuf from given Path (to avoid lifetime issues)
    let p = PathBuf::from(&path.to_str().unwrap());

    // let ft = get_file_type(path);

    let ft = if parent_is_symlink {
        ItemType::Symlink
    } else {
        get_file_type(path)
    };

    Ok(FileInfo {
        path: p,
        depth: depth,
        file_type: ft,
        size: md.len(),
        modified: modified.clone(),
        children: None,
        accessible: true,
    })
}

pub fn walk(
    path: &Path,
    depth: u8,
    sort_ascending: bool,
    skip_symlinks: bool,
    parent_is_symlink: bool,
    all_file_info: &mut Vec<FileInfo>,
) {
    let md = if skip_symlinks {
        fs::symlink_metadata(path)
    } else {
        fs::metadata(path)
    };

    let attr = match md {
        Ok(attr) => attr,
        Err(_) => {
            // println!("skipping {:#?}", path);
            return;
        }
    };

    let _parent_is_symlink = if parent_is_symlink {
        true
    } else {
        fs::symlink_metadata(path).unwrap().is_symlink()
    };

    if attr.is_file() {
        let fi = get_file_info(path, depth, &attr, _parent_is_symlink).unwrap();
        all_file_info.push(fi);
    } else if attr.is_dir() {
        let parent_info_idx = all_file_info.len();

        // variables to accumulate info about this dir
        let mut dir_info: Vec<FileInfo> = Vec::new();
        let mut total_size: u64 = 0;
        let mut most_recent: time::SystemTime = time::UNIX_EPOCH;

        // other info about the dir
        let p = PathBuf::from(&path.to_str().unwrap());
        let ft = if _parent_is_symlink {
            ItemType::Symlink
        } else {
            get_file_type(path)
        };

        let dir_iter_result = fs::read_dir(path);
        let accessible = match dir_iter_result {
            Err(_) => false,
            Ok(dir_iter) => {
                for entry in dir_iter {
                    let item: fs::DirEntry = entry.unwrap();

                    walk(
                        &item.path(),
                        depth + 1,
                        sort_ascending,
                        skip_symlinks,
                        _parent_is_symlink,
                        &mut dir_info,
                    );

                    let summarised_fi = dir_info.last().unwrap();
                    total_size += summarised_fi.size;

                    if summarised_fi.modified > most_recent {
                        most_recent = summarised_fi.modified;
                    }
                }
                true // dir is accessible
            }
        };

        // make FileInfo with summarised dir
        total_size += attr.len();

        if sort_ascending {
            dir_info.sort_by(|a, b| a.size.cmp(&b.size));
        } else {
            dir_info.sort_by(|a, b| b.size.cmp(&a.size));
        }

        let total_info = FileInfo {
            path: p,
            depth: depth,
            file_type: ft,
            size: total_size,
            modified: most_recent,
            children: Some(dir_info),
            accessible: accessible,
        };

        // insert parent dir entry above it's contents
        all_file_info.insert(parent_info_idx, total_info);
    }
}

pub fn list_files(cli: Cli) {
    let path = PathBuf::from(cli.file);

    let mut all_file_info: Vec<FileInfo> = Vec::new();
    walk(
        &path,
        1,
        cli.ascending,
        cli.skip_symlinks,
        false,
        &mut all_file_info,
    );

    print_results(
        &all_file_info,
        cli.humanize,
        cli.si,
        cli.time,
        cli.max_depth,
    );
}
