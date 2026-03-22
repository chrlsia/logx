use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "logx", about = "⚡ Universal log analyzer CLI", version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Read and display a log file
    Read {
        #[arg(required = true)]
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Read { file } => {
            println!("Will read: {}", file);
        }
    }
}