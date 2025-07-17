use clap::Parser;

mod db;
use db::Database;

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
    
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open database: {}", e);
            std::process::exit(1);
        }
    };
    
    match db.search(&cli.mode, cli.limit) {
        Ok(records) => {
            for record in records {
                println!("{}", record.command);
            }
        }
        Err(e) => {
            eprintln!("Search failed: {}", e);
            std::process::exit(1);
        }
    }
}
