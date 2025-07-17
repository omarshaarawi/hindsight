use clap::Parser;
use skim::prelude::*;
use std::io::Cursor;

mod db;
mod config;
use db::Database;
use config::Config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long)]
    mode: Option<String>,
    
    #[arg(long)]
    limit: Option<u32>,
}

fn main() {
    let cli = Cli::parse();
    let config = Config::load();
    
    let mode = cli.mode
        .or(config.default_mode)
        .unwrap_or_else(|| "global".to_string());
    let limit = cli.limit
        .or(config.default_limit)
        .unwrap_or(1000);
    
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
    
    let records = match db.search(&mode, limit, &current_session, &current_cwd) {
        Ok(records) => records,
        Err(e) => {
            eprintln!("Search failed: {}", e);
            std::process::exit(1);
        }
    };
    
    if records.is_empty() {
        eprintln!("No history found");
        std::process::exit(0);
    }
    
    let header = format!("Mode: {}", mode);
    let height = config.height.as_deref().unwrap_or("100%");
    let options = SkimOptionsBuilder::default()
        .height(Some(height))
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
