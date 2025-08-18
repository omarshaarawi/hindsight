use clap::{Parser, Subcommand};
use skim::prelude::*;

mod config;
mod db;
mod item;
mod stream;
use config::Config;
use db::Database;
use stream::StreamingSearch;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long)]
    mode: Option<String>,

    #[arg(long)]
    limit: Option<u32>,
}

#[derive(Subcommand)]
enum Commands {
    Init,
}

fn main() {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Init => match Database::new() {
                Ok(_) => {
                    println!(
                        "Database initialized successfully at: {:?}",
                        Database::db_path().unwrap()
                    );
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("Failed to initialize database: {}", e);
                    std::process::exit(1);
                }
            },
        }
    }

    let config = Config::load();

    let mut mode = cli
        .mode
        .or(config.default_mode)
        .unwrap_or_else(|| "global".to_string());
    let limit = cli.limit.or(config.default_limit).unwrap_or(1000);

    let current_session = std::env::var("HINDSIGHT_SESSION").unwrap_or_default();
    let current_cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let _db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    let mut selected_cmd: Option<String> = None;
    let mut edit = false;

    loop {
        let header = format!("Mode: {}", mode);
        let height = config.height.as_deref().unwrap_or("100%").to_string();
        let options = SkimOptionsBuilder::default()
            .height(height)
            .multi(false)
            .reverse(true)
            .bind(vec!["tab:accept".to_string(), "ctrl-r:accept".to_string()])
            .header(Some(header))
            .build()
            .unwrap();

        let search = StreamingSearch::new(
            mode.clone(),
            limit,
            current_session.clone(),
            current_cwd.clone(),
        );

        let items = search.into_receiver();

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
        print!("__HINDSIGHT_MODE__{}__", mode);
        if edit {
            print!("__HINDSIGHT_EDIT__");
        }
        print!("{}", cmd);
    }
}
