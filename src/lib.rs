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

use clap::Parser;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time;
use std::vec;
use std::cmp::Reverse;

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

enum ByteType {
    Binary,
    Decimal,
}

#[derive(Debug)]
pub enum ItemType {
    File,
    Dir,
    Symlink,
}

#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub depth: u8,
    pub file_type: ItemType,
    pub size: u64,
    pub modified: time::SystemTime,
}

fn format_size(size: u64, byte_type: ByteType) -> String {
    let mut size_f = size as f64;
    let mut prefixes = vec!["", "K", "M", "G", "T"];

    let div = match byte_type {
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

    format!("{0:.3} {1}B", size_f, prefixes[idx])
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

fn sort(path_info: &mut Vec<FileInfo>, ascending: bool, field: &str) {
    // modify path_info in place
    // sort by field string from FileInfo struct
    // see Vector.sort_by_key
    // https://doc.rust-lang.org/std/vec/struct.Vec.html#method.sort_by_key
    if ascending {
        path_info.sort_by_key(|item| item.size);
    }
    else {
        path_info.sort_by_key(|item| Reverse(item.size));
    };
    

}

fn print_results(path_info: &Vec<FileInfo>) {
    println!("\n\n{:#?}", path_info);
}

pub fn walk(path: &Path, depth: u8, indent: Option<usize>) -> Result<Vec<FileInfo>, Box<dyn Error>> { //Result<FileInfo, Box<dyn Error>> { //
    let indent = indent.unwrap_or(0);

    let mut all_file_info: Vec<FileInfo> = Vec::new();

    println!(
        "{}walking {:?}...",
        " ".repeat(indent),
        &path.to_str().unwrap()
    );

    let attr = fs::metadata(path)?;
    if attr.is_file() {
        let fi = get_file_info(path, depth, &attr)?;

        println!("{}is file of size {}", " ".repeat(indent), fi.size);
        println!("{}pushing file {:?} to vec", " ".repeat(indent), path);
        all_file_info.push(fi);

    } else if attr.is_dir() {
        println!("{}is dir, iterating...", " ".repeat(indent));

        let mut total_size: u64 = 0;
        let mut most_recent: time::SystemTime = time::UNIX_EPOCH;
        
        for entry in fs::read_dir(path)? {
            let item = entry?;
            let mut fi = walk(&item.path(), depth+1, Some(indent + 2))?;

            println!("{}appending vec of size {} to vec", " ".repeat(indent), fi.len());
            all_file_info.append(&mut fi);

            let summarised_fi = all_file_info.last().unwrap();

            println!(
                "{}adding size {} from '{}'",
                " ".repeat(indent),
                summarised_fi.size,
                &item.path().file_name().unwrap().to_str().unwrap()
            );
            total_size += summarised_fi.size;
            println!("{}total size: {}", " ".repeat(indent), total_size);
            if summarised_fi.modified > most_recent {
                most_recent = summarised_fi.modified;
            }
        }

        // make FileInfo with summarised dir?
        // after max depth, only include summary
        println!(
            "{}adding size {} from dir '{}'",
            " ".repeat(indent),
            attr.len(),
            &path.file_name().unwrap().to_str().unwrap()
        );
        total_size += attr.len();
        println!("{}total size: {}", " ".repeat(indent), total_size);

        let p = PathBuf::from(&path.to_str().unwrap());
        let ft = get_file_type(&attr);
        let total_info = FileInfo {
            path: p,
            depth: depth,
            file_type: ft,
            size: total_size,
            modified: most_recent,
        };

        all_file_info.push(total_info);
    }

    Ok(all_file_info)
}

pub fn list_files(cli: Cli) {
    let path = PathBuf::from(cli.file);

    // let path_info = walk(&path, None).unwrap();

    let mut total_info = walk(&path, 1, None).unwrap();
    // let path_info = total_info.last().unwrap().clone();
    sort(&mut total_info, cli.ascending, "size");

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

    print_results(&total_info);
}
