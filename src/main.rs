mod remote;

use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[clap(
    version = "1.0",
    author = "Erik Karsten",
    about = "Snoop who ran ansibles"
)]
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

async fn aquire_data(source: Source) -> Vec<String> {
    match source {
        Source::File(path) => {
            let contents =
                std::fs::read_to_string(path).expect("Something went wrong reading the file");
            contents.lines().map(|s| s.to_string()).collect()
        }

        Source::Remote(host) => remote::read_remote_auth_log(&host).await.expect("Something went wrong reading the remote file")
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let local_timezone = chrono::Local::now().offset();

    let source = determine_source(args.source);

    let data = aquire_data(source).await;

    // Print how many lines were read
    println!("Read {} lines", data.len());
}
