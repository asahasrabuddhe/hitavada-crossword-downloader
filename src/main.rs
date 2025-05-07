use anyhow::{Context, Result};
use chrono::Local;
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use scraper::{Html, Selector};
use std::fs;

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

fn main() -> Result<()> {
    // Get today's date in yyyy-mm-dd format
    let today = Local::now().format("%Y-%m-%d").to_string();
    let today_str = today.as_str();
    
    // Create a client with a user agent to mimic a browser
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36")
        .build()?;

    // Create headers
    let headers = create_headers()?;

    // Construct the mapping coordinates request
    let mapping_url = "https://www.ehitavada.com/val.php";
    let mapping_data = format!(
        "get_mapping_coords=https%3A%2F%2Fehitavada.com%2Fencyc%2F6%2F{}{}{}%2FMpage_2.jpg&get_mapping_coords_date={}&get_mapping_coords_prefix=Mpage&get_mapping_coords_page=2",
        &today_str[0..4], // year
        &today_str[5..7], // month
        &today_str[8..10], // day
        today
    );

    // Get the mapping coordinates
    let mapping_response = client
        .post(mapping_url)
        .headers(headers.clone())
        .body(mapping_data)
        .send()?;
    println!("Mapping response status: {}", mapping_response.status());

    let mapping_html = mapping_response.text()?;
    println!("Mapping HTML content length: {} bytes", mapping_html.len());

    // Parse the mapping HTML
    let mapping_document = Html::parse_document(&mapping_html);
    let area_selector = Selector::parse("area").unwrap();
    let areas: Vec<_> = mapping_document.select(&area_selector).collect();
    println!("Found {} area elements", areas.len());

    // Get the href from the third area element
    let third_area = areas.get(2)
        .context("Could not find third area element")?;
    let href = third_area.value().attr("href")
        .context("Could not find href attribute")?;

    // Construct the full URL for the crossword page
    let crossword_url = format!("https://www.ehitavada.com/{}", href);
    println!("Crossword URL: {}", crossword_url);

    // Download the crossword page
    let crossword_response = client
        .get(&crossword_url)
        .headers(headers.clone())
        .send()?;
    println!("Crossword page status: {}", crossword_response.status());

    let crossword_html = crossword_response.text()?;
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
        .send()?;
    println!("Image download status: {}", img_response.status());

    // Save the image
    let img_data = img_response.bytes()?;
    let filename = format!("crossword_{}.jpg", today);
    fs::write(&filename, img_data)?;
    println!("Image saved as: {}", filename);

    Ok(())
}
