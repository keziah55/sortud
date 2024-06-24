// print size of files and dirs 
// use different colours for files and dirs
// show total at bottom
// print in nice table format

use clap::Parser;

fn main() {
    let cli = sortud::Cli::parse();
    sortud::list_files(cli)
}
