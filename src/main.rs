use anyhow::Result;
use chrono::{Local, NaiveDate};
use clap::Parser;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use reqwest::Client;

mod drive;
mod http;
mod parser;
mod types;
mod crossword;

use types::{LambdaInput, LambdaOutput};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Date in YYYY-MM-DD format (defaults to today)
    #[arg(short, long, value_parser = types::parse_date)]
    date: Option<NaiveDate>,
}

async fn handler(event: LambdaEvent<LambdaInput>) -> Result<LambdaOutput, Error> {
    let date = match event.payload.date {
        Some(date_str) => NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|e| anyhow::anyhow!("Invalid date format: {}", e))?,
        None => Local::now().date_naive(),
    };

    // Create a client with a user agent to mimic a browser
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36")
        .build()?;

    let filename = crossword::download_crossword(&client, date).await?;
    
    Ok(LambdaOutput {
        message: "Crossword downloaded successfully".to_string(),
        filename,
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(handler)).await
}
