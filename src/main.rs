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
    
    let mut mode = cli.mode
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
    
    let mut selected_cmd: Option<String> = None;
    let mut edit = false;
    
    loop {
        let records = match db.search(&mode, limit, &current_session, &current_cwd) {
            Ok(records) => records,
            Err(e) => {
                eprintln!("Search failed: {}", e);
                std::process::exit(1);
            }
        };
        
        if records.is_empty() {
            break;
        }
        
        let header = format!("Mode: {}", mode);
        let height = config.height.as_deref().unwrap_or("100%");
        let options = SkimOptionsBuilder::default()
            .height(Some(height))
            .multi(false)
            .reverse(true)
            .bind(vec!["tab:accept", "ctrl-r:accept"])
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
            if output.is_abort {
                break;
            }
            
            if output.final_key == Key::Tab {
                if let Some(item) = output.selected_items.first() {
                    selected_cmd = Some(item.output().to_string());
                    edit = true;
                }
                break;
            } else if output.final_key == Key::Ctrl('r') {
                mode = match mode.as_str() {
                    "global" => "session".to_string(),
                    "session" => "cwd".to_string(),
                    _ => "global".to_string(),
                };
                continue;
            } else {
                if let Some(item) = output.selected_items.first() {
                    selected_cmd = Some(item.output().to_string());
                }
                break;
            }
        } else {
            break;
        }
    }
    
    if let Some(cmd) = selected_cmd {
        if edit {
            print!("__HINDSIGHT_EDIT__");
        }
        print!("{}", cmd);
    }
}
