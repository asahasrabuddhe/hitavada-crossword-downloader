use anyhow::{Context, Result};
use chrono::NaiveDate;
use std::fs;
use reqwest::Client;
use scraper::{Html, Selector};

use crate::http;
use crate::parser;
use crate::drive;

pub async fn download_crossword(date: NaiveDate) -> Result<String> {
    let date_str = date.format("%Y-%m-%d").to_string();
    let date_str_slice = date_str.as_str();
    
    // Create a client with a user agent to mimic a browser
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36")
        .build()?;

    // Create headers
    let headers = http::create_headers()?;

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
        if let Some(href) = parser::get_target_rect(&mapping_html) {
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
            let google_credentials = drive::get_google_credentials().await?;

            // Upload to Google Drive
            let file_id = drive::upload_to_drive(&filename, &google_credentials).await?;
            println!("File uploaded to Google Drive with ID: {}", file_id);

            return Ok(filename);
        }

        println!("Target area not found on page {}, trying next page...", page);
    }

    Err(anyhow::anyhow!("Could not find crossword on any page"))
} 