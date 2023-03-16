use clap::Parser;

#[derive(Parser)]
#[clap(version = "1.0", author = "Erik Karsten", about = "Snoop who ran ansibles")]
struct Args {
    host: String
}

fn main() {
    let args = Args::parse();

    println!("Host is: {}", args.host);
}
