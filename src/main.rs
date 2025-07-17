use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, default_value = "global")]
    mode: String,
    
    #[arg(long, default_value_t = 1000)]
    limit: u32,
}

fn main() {
    let cli = Cli::parse();
    println!("Mode: {}, Limit: {}", cli.mode, cli.limit);
}
