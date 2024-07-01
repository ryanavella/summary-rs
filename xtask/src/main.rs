use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Download bench data
    DownloadBenchData,
}

fn main() {
    let Args { command } = Args::parse();
    match command {
        Command::DownloadBenchData => {
            let body: String = ureq::get("https://gutenberg.org/cache/epub/1513/pg1513.txt")
                .call()
                .expect("unable to download bench data from gutenberg.org")
                .into_string()
                .expect("unexpected data from gutenberg.org");
            if let Err(err) = std::fs::create_dir("benches/gutenberg") {
                if err.kind() != std::io::ErrorKind::AlreadyExists {
                    todo!();
                }
            }
            let () = std::fs::write("benches/gutenberg/1513.txt", &body).unwrap();
        }
    }
}
