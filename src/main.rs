use clap::Parser;
use skim::prelude::*;
use std::io::Cursor;

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
    
    let current_session = std::env::var("HINDSIGHT_SESSION").unwrap_or_default();
    let current_cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open database: {}", e);
            std::process::exit(1);
        }
    };
    
    let records = match db.search(&cli.mode, cli.limit, &current_session, &current_cwd) {
        Ok(records) => records,
        Err(e) => {
            eprintln!("Search failed: {}", e);
            std::process::exit(1);
        }
    };
    
    let header = format!("Mode: {}", cli.mode);
    let options = SkimOptionsBuilder::default()
        .height(Some("100%"))
        .multi(false)
        .reverse(true)
        .bind(vec!["ctrl-r:accept"])
        .header(Some(&header))
        .build()
        .unwrap();
    
    let input = records
        .iter()
        .map(|r| r.command.clone())
        .collect::<Vec<_>>()
        .join("\n");
    
    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(input));
    
    if let Some(output) = Skim::run_with(&options, Some(items)) {
        if !output.is_abort {
            if let Some(item) = output.selected_items.first() {
                print!("{}", item.output());
            }
        }
    }
}
