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

fn aquire_data(source: Source) -> Vec<String> {
    match source {
        Source::File(path) => {
            let contents =
                std::fs::read_to_string(path).expect("Something went wrong reading the file");
            contents.lines().map(|s| s.to_string()).collect()
        }

        Source::Remote(host) => {
            let mut cmd = std::process::Command::new("ssh");
            cmd.arg(host).arg("cat /var/log/auth.log");
            let output = cmd.output().expect("failed to execute process");
            let contents =
                String::from_utf8(output.stdout).expect("failed to convert output to string");
            contents.lines().map(|s| s.to_string()).collect()
        }
    }
}

fn main() {
    let args = Args::parse();

    let local_timezone = chrono::Local::now().offset();

    let source = determine_source(args.source);

    let data = aquire_data(source);

    println!("{:?}", data);
}
