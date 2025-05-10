use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use clap::Parser;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use reqwest::{
    header::{HeaderMap, HeaderValue},
};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs;
use std::env;
use aws_sdk_ssm::Client as SsmClient;
use aws_config::BehaviorVersion;
use google_drive3::DriveHub;
use yup_oauth2::ServiceAccountAuthenticator;
use std::path::Path;
use std::io::Cursor;
use hyper::Client;

#[derive(Serialize, Deserialize)]
struct LambdaInput {
    date: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct LambdaOutput {
    message: String,
    filename: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Date in YYYY-MM-DD format (defaults to today)
    #[arg(short, long, value_parser = parse_date)]
    date: Option<NaiveDate>,
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format. Please use YYYY-MM-DD: {}", e))
}

async fn get_google_credentials() -> Result<String> {
    // In local development, read from file
    if let Ok(path) = env::var("GOOGLE_SERVICE_ACCOUNT_PATH") {
        return fs::read_to_string(path)
            .context("Failed to read Google service account file");
    }

    // In Lambda, get from SSM Parameter Store
    let config = aws_config::defaults(BehaviorVersion::latest())
        .load()
        .await;
    
    let client = SsmClient::new(&config);
    
    let parameter = client
        .get_parameter()
        .name("/hitavada-crossword/google-service-account")
        .with_decryption(true)
        .send()
        .await?;
    
    let value = parameter.parameter()
        .and_then(|p| p.value())
        .context("Parameter value is empty")?;
    
    Ok(value.to_string())
}

fn create_headers() -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert("accept", HeaderValue::from_static("*/*"));
    headers.insert("accept-language", HeaderValue::from_static("en-GB,en-US;q=0.9,en;q=0.8"));
    headers.insert("content-type", HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"));
    headers.insert("dnt", HeaderValue::from_static("1"));
    headers.insert("origin", HeaderValue::from_static("https://www.ehitavada.com"));
    headers.insert("priority", HeaderValue::from_static("u=1, i"));
    headers.insert("sec-ch-ua", HeaderValue::from_static("\"Not.A/Brand\";v=\"99\", \"Chromium\";v=\"136\""));
    headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    headers.insert("sec-ch-ua-platform", HeaderValue::from_static("\"macOS\""));
    headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
    headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
    headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
    headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36"));
    headers.insert("x-requested-with", HeaderValue::from_static("XMLHttpRequest"));
    Ok(headers)
}

async fn upload_to_drive(filename: &str, credentials: &str) -> Result<String> {
    let folder_id = env::var("GOOGLE_DRIVE_FOLDER_ID")
        .context("GOOGLE_DRIVE_FOLDER_ID environment variable not set")?;

    // Create authenticator
    let sa_key = serde_json::from_str(credentials)?;
    let auth = ServiceAccountAuthenticator::builder(sa_key)
        .build()
        .await?;

    // Create Drive client with hyper
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_only()
        .enable_http1()
        .build();
    
    let client = Client::builder()
        .build(https);

    let hub = DriveHub::new(client, auth);

    // Read file
    let file_content = fs::read(filename)?;
    let file_name = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid filename")?;

    // Create file metadata
    let file = google_drive3::api::File {
        name: Some(file_name.to_string()),
        parents: Some(vec![folder_id]),
        ..Default::default()
    };

    // Upload file using Cursor
    let cursor = Cursor::new(file_content);
    let (_, file) = hub
        .files()
        .create(file)
        .upload(cursor, "image/jpeg".parse()?)
        .await?;

    Ok(file.id.unwrap_or_default())
}

async fn download_crossword(date: NaiveDate) -> Result<String> {
    let date_str = date.format("%Y-%m-%d").to_string();
    let date_str_slice = date_str.as_str();
    
    // Create a client with a user agent to mimic a browser
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36")
        .build()?;

    // Create headers
    let headers = create_headers()?;

    // Construct the mapping coordinates request
    let mapping_url = "https://www.ehitavada.com/val.php";
    let mapping_data = format!(
        "get_mapping_coords=https%3A%2F%2Fehitavada.com%2Fencyc%2F6%2F{}{}{}%2FMpage_2.jpg&get_mapping_coords_date={}&get_mapping_coords_prefix=Mpage&get_mapping_coords_page=2",
        &date_str_slice[0..4], // year
        &date_str_slice[5..7], // month
        &date_str_slice[8..10], // day
        date_str
    );

    // Get the mapping coordinates
    let mapping_response = client
        .post(mapping_url)
        .headers(headers.clone())
        .body(mapping_data)
        .send()
        .await?;
    println!("Mapping response status: {}", mapping_response.status());

    let mapping_html = mapping_response.text().await?;
    println!("Mapping HTML content length: {} bytes", mapping_html.len());

    // Parse the mapping HTML
    let mapping_document = Html::parse_document(&mapping_html);
    let area_selector = Selector::parse("area").unwrap();
    let areas: Vec<_> = mapping_document.select(&area_selector).collect();
    println!("Found {} area elements", areas.len());

    // Get the href from the second-to-last area element
    let second_last_area = areas.get(areas.len() - 1)
        .context("Could not find second-to-last area element")?;
    let href = second_last_area.value().attr("href")
        .context("Could not find href attribute")?;

    // Construct the full URL for the crossword page
    let crossword_url = format!("https://www.ehitavada.com/{}", href);
    println!("Crossword URL: {}", crossword_url);

    // Download the crossword page
    let crossword_response = client
        .get(&crossword_url)
        .headers(headers.clone())
        .send()
        .await?;
    println!("Crossword page status: {}", crossword_response.status());

    let crossword_html = crossword_response.text().await?;
    println!("Crossword HTML content length: {} bytes", crossword_html.len());

    // Parse the crossword page
    let crossword_document = Html::parse_document(&crossword_html);
    
    // Find the image URL
    let img_selector = Selector::parse(".slices_container img").unwrap();
    let img = crossword_document.select(&img_selector).next()
        .context("Could not find crossword image")?;
    
    let img_src = img.value().attr("src")
        .context("Could not find image source")?;
    
    let img_url = format!("https://www.ehitavada.com/{}", img_src);
    println!("Image URL: {}", img_url);

    // Download the image
    let img_response = client
        .get(&img_url)
        .headers(headers)
        .send()
        .await?;
    println!("Image download status: {}", img_response.status());

    // Save the image
    let img_data = img_response.bytes().await?;
    let filename = format!("/tmp/crossword_{}.jpg", date_str);
    fs::write(&filename, img_data)?;
    println!("Image saved as: {}", filename);

    // Get Google credentials
    let google_credentials = get_google_credentials().await?;

    // Upload to Google Drive
    let file_id = upload_to_drive(&filename, &google_credentials).await?;
    println!("File uploaded to Google Drive with ID: {}", file_id);

    Ok(filename)
}

async fn handler(event: LambdaEvent<LambdaInput>) -> Result<LambdaOutput, Error> {
    let date = match event.payload.date {
        Some(date_str) => NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|e| anyhow::anyhow!("Invalid date format: {}", e))?,
        None => Local::now().date_naive(),
    };

    let filename = download_crossword(date).await?;
    
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
