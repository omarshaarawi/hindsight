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
    
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open database: {}", e);
            std::process::exit(1);
        }
    };
    
    let records = match db.search(&cli.mode, cli.limit) {
        Ok(records) => records,
        Err(e) => {
            eprintln!("Search failed: {}", e);
            std::process::exit(1);
        }
    };
    
    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(false)
        .build()
        .unwrap();
    
    let input = records
        .iter()
        .map(|r| r.command.clone())
        .collect::<Vec<_>>()
        .join("\n");
    
    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(input));
    
    let selected = Skim::run_with(&options, Some(items))
        .map(|out| out.selected_items)
        .unwrap_or_else(Vec::new);
    
    if let Some(item) = selected.first() {
        print!("{}", item.output());
    }
}
