mod remote;
mod parser;
mod entry;

use clap::Parser;
use std::path::PathBuf;

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

async fn aquire_data(source: Source) -> anyhow::Result<Vec<String>> {
    match source {
        Source::File(path) => {
            let contents =
                std::fs::read_to_string(path)?;
            Ok(contents.lines().map(|s| s.to_string()).collect())
        }

        Source::Remote(host) => remote::read_remote_auth_log(&host).await
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let local_timezone = chrono::Local::now().offset();

    let source = determine_source(args.source);

    let data = match aquire_data(source).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Print how many lines were read
    println!("Read {} lines", data.len());

    let parsed_lines = parser::parse_lines(data);
    let ansible_runs = parser::get_ansible_runs(parsed_lines);

    // Print how many ansible runs were found
    println!("Found {} ansible runs", ansible_runs.len());

    dbg!(ansible_runs);

}
