// if file, return size and modified date
// if dir, walk and call func on all items

// sum sizes, return most recent mod date

// use map() function to apply (recursive) func to all in top-level
// can this be modified to both sum sizes and find most recent time (on a per-dir basis)?

// nb: there is a max-depth arg
// this should be used when showing results
// but for calculating dir sizes, need to walk all the way down

// print results in nice table
// use different colours for files and dirs

// future features:
// - follow or ignore symlinks
// - exclude patterns
// - sort by size, modified date or name (option for dirs first)

// Metadata docs https://doc.rust-lang.org/std/fs/struct.Metadata.html

use chrono::{DateTime, Utc};
use clap::Parser;
use std::cmp::{Ordering, Reverse};
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
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
    #[arg(long)]
    time: bool,

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

// impl Ord for ItemType {
//     #[inline]
//     fn cmp(&self, other: &ItemType) -> Ordering {
//         if self == other {
//             Ordering::Equal
//         } else if self == ItemType.Dir {
//             Ordering::Greater
//         } else {
//             Ordering::Less
//         }

//     }
// }

#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub depth: u8,
    pub file_type: ItemType,
    pub size: u64,
    pub modified: time::SystemTime,
}

impl FileInfo {
    pub fn to_string(&self, humanize: bool, byte_type: &ByteType, size_width: usize) -> String {
        let size = if humanize {
            format_size(self.size, byte_type, size_width)
        } else {
            format!("{:>width$}", self.size, width = size_width)
        };

        let date_time: DateTime<Utc> = self.modified.clone().into();
        let ts = format!("{}", date_time.format("%Y %b %d %H:%M:%S"));

        let s = format!("{}  {}  {}", size, ts, self.path.to_str().unwrap());

        match self.file_type {
            ItemType::Dir => format!("\x1b[34m{:#}\x1b[0m", s),
            ItemType::Symlink => format!("\x1b[92m{:#}\x1b[0m", s),
            ItemType::File => format!("{:#}", s),
        }
    }
}
impl fmt::Display for FileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.to_string(true, &ByteType::Binary, 10);
        write!(f, "{}", s)
    }
}

// impl FileInfo {
//     fn get_field<T>(&self, field: &str) -> &T {
//         match field {
//             "path" => &self.path,
//             "depth" => &self.depth,
//             "file_type" => &self.file_type,
//             "size" => &self.size,
//             "modified" => &self.modified,
//         }
//     }
// }

fn format_size(size: u64, byte_type: &ByteType, size_width: usize) -> String {
    let mut size_f = size as f64;
    let mut prefixes = vec!["", "K", "M", "G", "T"];

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

    let width = (size_width % 3) + 4;

    format!("{0:width$.3} {1}B", size_f, prefixes[idx])
}

fn get_file_type(md: &fs::Metadata) -> ItemType {
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

fn get_file_info(path: &Path, depth: u8, md: &fs::Metadata) -> Result<FileInfo, Box<dyn Error>> {
    let modified = md.modified()?;
    // make new PathBuf from given Path (to avoid lifetime issues)
    let p = PathBuf::from(&path.to_str().unwrap());

    let ft = get_file_type(md);

    Ok(FileInfo {
        path: p,
        depth: depth,
        file_type: ft,
        size: md.len(),
        modified: modified.clone(),
    })
}

// #[derive(PartialEq, Eq, Debug, Copy, Default, Hash, PartialOrd, Ord, Clone)]
// pub struct NonReverse<T>(pub T);

// fn sort(path_info: &mut Vec<FileInfo>, ascending: bool, field: &str) {
//     // modify path_info in place
//     // sort by field string from FileInfo struct
//     // see Vector.sort_by_key
//     // https://doc.rust-lang.org/std/vec/struct.Vec.html#method.sort_by_key

//     match field {
//         "path" => {
//             if ascending {
//                 path_info.sort_by_key(|item: &FileInfo| item.path)
//             }
//             else {
//                 path_info.sort_by_key(|item| Reverse(item.path))
//             }
//         },
//         "depth" => if ascending {
//             path_info.sort_by_key(|item| item.depth)
//         }
//         else {
//             path_info.sort_by_key(|item| Reverse(item.depth))
//         },
//         "file_type" => if ascending {
//             path_info.sort_by_key(|item| item.file_type)
//         }
//         else {
//             path_info.sort_by_key(|item| Reverse(item.file_type))
//         },
//         "size" => {
//             if ascending {
//                 path_info.sort_by_key(|item| item.size)
//             }
//             else {
//                 path_info.sort_by_key(|item| Reverse(item.size))
//             }
//         },
//         "modified" => if ascending {
//             path_info.sort_by_key(|item| item.modified)
//         }
//         else {
//             path_info.sort_by_key(|item| Reverse(item.modified))
//         },
//     }

//     // let f = if ascending {
//     //     NonReverse
//     // } else {
//     //     Reverse
//     // };

//     // path_info.sort_by_key(|item: &FileInfo| f(item.size));

//     if ascending {
//         path_info.sort_by_key(|item| item.size);
//     }
//     else {
//         path_info.sort_by_key(|item| Reverse(item.size));
//     };

// }

fn print_results(path_info: &Vec<FileInfo>, humanize: bool, si: bool) {
    let max_size = path_info.iter().max_by_key(|info| info.size).unwrap().size;
    let digits = format!("{}", max_size).len();

    let byte_type = if si {
        ByteType::Decimal
    } else {
        ByteType::Binary
    };

    for info in path_info {
        let s = info.to_string(humanize, &byte_type, digits);
        println!("{}", s)
    }
}

pub fn walk(
    path: &Path,
    depth: u8,
    indent: Option<usize>,
) -> Result<Vec<FileInfo>, Box<dyn Error>> {
    let indent = indent.unwrap_or(0);

    let mut all_file_info: Vec<FileInfo> = Vec::new();

    let attr = fs::metadata(path)?;
    if attr.is_file() {
        let fi = get_file_info(path, depth, &attr)?;
        all_file_info.push(fi);
    } else if attr.is_dir() {
        let parent_info_idx = all_file_info.len();

        let mut total_size: u64 = 0;
        let mut most_recent: time::SystemTime = time::UNIX_EPOCH;

        for entry in fs::read_dir(path)? {
            let item: fs::DirEntry = entry?;
            let mut fi = walk(&item.path(), depth + 1, Some(indent + 2))?;

            all_file_info.append(&mut fi);

            let summarised_fi = all_file_info.last().unwrap();
            total_size += summarised_fi.size;
            // println!("{}total size: {}", " ".repeat(indent), total_size);
            if summarised_fi.modified > most_recent {
                most_recent = summarised_fi.modified;
            }
        }

        // make FileInfo with summarised dir
        total_size += attr.len();

        let p = PathBuf::from(&path.to_str().unwrap());
        let ft = get_file_type(&attr);
        let total_info = FileInfo {
            path: p,
            depth: depth,
            file_type: ft,
            size: total_size,
            modified: most_recent,
        };

        // insert parent dir entry above it's contents
        all_file_info.insert(parent_info_idx, total_info);
    }

    Ok(all_file_info)
}

pub fn list_files(cli: Cli) {
    let path = PathBuf::from(cli.file);

    // let path_info = walk(&path, None).unwrap();

    let mut total_info = walk(&path, 1, None).unwrap();
    // let path_info = total_info.last().unwrap().clone();
    // sort(&mut total_info, cli.ascending, "size");

    // let byte_type = if cli.si {
    //     ByteType::Decimal
    // } else {
    //     ByteType::Binary
    // };
    // let format_size = format_size(path_info.size, byte_type);

    // println!(
    //     "\nDone!\ntotal size: {},\nmost recent: {:?}",
    //     format_size, path_info.modified
    // );

    // println!("\n\n{:#?}", total_info)

    print_results(&total_info, cli.humanize, cli.si);
}
