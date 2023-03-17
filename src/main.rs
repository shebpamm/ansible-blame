use clap::Parser;
use std::path::{Path,PathBuf};

#[derive(Parser)]
#[clap(version = "1.0", author = "Erik Karsten", about = "Snoop who ran ansibles")]
struct Args {
    source: String,
}

enum Source {
    File(PathBuf),
    Remote(String),
}

fn determine_source(source: String) -> Source {
    // Check if source is a local file
    let path = PathBuf::from(source.clone());
    match path.is_file() {
        true => Source::File(path),
        false => Source::Remote(source),
    }
}

fn main() {
    let args = Args::parse();

    let local_timezone = chrono::Local::now().offset();

    match determine_source(args.source) {
        Source::File(path) => println!("File is: {}", path.display()),
        Source::Remote(host) => println!("Host is: {}", host),
    }
}
