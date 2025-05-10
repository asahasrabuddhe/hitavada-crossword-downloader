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

#[derive(Debug, PartialEq)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
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

/// Parses a single coords string into a Rect
fn parse_coords(coords_str: &str) -> Option<Rect> {
    let parts: Vec<i32> = coords_str
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    if parts.len() == 4 {
        Some(Rect {
            x1: parts[0],
            y1: parts[1],
            x2: parts[2],
            y2: parts[3],
        })
    } else {
        None
    }
}

/// Gets the target area's href from the HTML content with a tolerance of 50 for y1 and y2, and 10 for x2
fn get_target_rect(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let area_selector = Selector::parse("area").unwrap();
    let tolerance_y1 = 50;
    let tolerance_x2 = 10;
    let tolerance_y2 = 50;

    document.select(&area_selector)
        .find_map(|area| {
            if let Some(coords) = area.value().attr("coords") {
                if let Some(rect) = parse_coords(coords) {
                    // Check if coordinates are within tolerance
                    let y1_in_range = (rect.y1 - 1625).abs() <= tolerance_y1;
                    let x2_in_range = (rect.x2 - 1000).abs() <= tolerance_x2;
                    let y2_in_range = (rect.y2 - 2775).abs() <= tolerance_y2;
                    
                    if rect.x1 == 0 && y1_in_range && x2_in_range && y2_in_range {
                        area.value().attr("href").map(String::from)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
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

    // Try pages 1 through 20
    for page in 1..=20 {
        // Construct the mapping coordinates request
        let mapping_url = "https://www.ehitavada.com/val.php";
        let mapping_data = format!(
            "get_mapping_coords=https%3A%2F%2Fehitavada.com%2Fencyc%2F6%2F{}{}{}%2FMpage_{}.jpg&get_mapping_coords_date={}&get_mapping_coords_prefix=Mpage&get_mapping_coords_page={}",
            &date_str_slice[0..4], // year
            &date_str_slice[5..7], // month
            &date_str_slice[8..10], // day
            page,
            date_str,
            page
        );

        // Get the mapping coordinates
        let mapping_response = client
            .post(mapping_url)
            .headers(headers.clone())
            .body(mapping_data)
            .send()
            .await?;
        println!("Mapping response status for page {}: {}", page, mapping_response.status());

        let mapping_html = mapping_response.text().await?;
        println!("Mapping HTML content length for page {}: {} bytes", page, mapping_html.len());

        // Get the target area's href
        if let Some(href) = get_target_rect(&mapping_html) {
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

            return Ok(filename);
        }

        println!("Target area not found on page {}, trying next page...", page);
    }

    Err(anyhow::anyhow!("Could not find crossword on any page"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_target_rect_exact_match() {
        let html = r#"
            <map>
                <area shape="rect" coords="0,1625,1000,2775" href="test1"/>
            </map>
        "#;
        assert_eq!(get_target_rect(html), Some("test1".to_string()));
    }

    #[test]
    fn test_get_target_rect_within_tolerance() {
        let html = r#"
            <map>
                <area shape="rect" coords="0,1670,1001,2764" href="test2"/>
            </map>
        "#;
        assert_eq!(get_target_rect(html), Some("test2".to_string()));
    }

    #[test]
    fn test_get_target_rect_outside_tolerance() {
        let html = r#"
            <map>
                <area shape="rect" coords="0,1627,242,2286" href="test3"/>
            </map>
        "#;
        assert_eq!(get_target_rect(html), None);
    }

    #[test]
    fn test_get_target_rect_wrong_x1() {
        let html = r#"
            <map>
                <area shape="rect" coords="10,1625,1000,2775" href="test4"/>
            </map>
        "#;
        assert_eq!(get_target_rect(html), None);
    }

    #[test]
    fn test_get_target_rect_multiple_areas() {
        let html = r#"
            <map>
                <area shape="rect" coords="0,100,500,1000" href="test5"/>
                <area shape="rect" coords="0,1670,1001,2764" href="test6"/>
                <area shape="rect" coords="0,2000,500,3000" href="test7"/>
            </map>
        "#;
        assert_eq!(get_target_rect(html), Some("test6".to_string()));
    }

    #[test]
    fn test_get_target_rect_invalid_coords() {
        let html = r#"
            <map>
                <area shape="rect" coords="invalid" href="test8"/>
                <area shape="rect" coords="0,1625,1000" href="test9"/>
            </map>
        "#;
        assert_eq!(get_target_rect(html), None);
    }

    #[test]
    fn test_get_target_rect_empty_html() {
        let html = "";
        assert_eq!(get_target_rect(html), None);
    }

    #[test]
    fn test_get_target_rect_no_areas() {
        let html = r#"
            <map>
            </map>
        "#;
        assert_eq!(get_target_rect(html), None);
    }
}
