mod reader;
mod parser;
mod entry;

use reader::SourceReader;
use clap::Parser;
use std::path::PathBuf;
use std::io::{self, Write};

use rpassword::read_password;

#[derive(Parser)]
#[clap(
    version = "1.0",
    author = "Erik Karsten",
    about = "Snoop who ran ansibles"
)]
struct Args {
    source: String,

    #[clap(short = 'p', long)]
    ask_sudo: bool,
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let password: Option<String> = match args.ask_sudo {
        true => {
            print!("Enter sudo password: ");
            io::stdout().flush().unwrap();
            let password = read_password().unwrap();
            Some(password)
        }
        false => None,
    };

    let local_timezone = chrono::Local::now().offset();

    let source = determine_source(args.source);

    let source_reader = match source {
        Source::File(path) => SourceReader::Local(reader::LocalSource::new(path)),
        Source::Remote(host) => SourceReader::Remote(reader::RemoteSource::new(host, password)),
    };

    let data = match source_reader.read().await {
        Ok(data) => data,
        Err(e) => {
            println!("Error: {}", e);
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
