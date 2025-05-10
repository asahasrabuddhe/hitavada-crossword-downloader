use anyhow::{Context, Result};
use std::fs;
use std::env;
use std::path::Path;
use std::io::Cursor;
use aws_sdk_ssm::Client as SsmClient;
use aws_config::BehaviorVersion;
use google_drive3::DriveHub;
use yup_oauth2::ServiceAccountAuthenticator;
use hyper::Client;

pub async fn get_google_credentials() -> Result<String> {
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

pub async fn upload_to_drive(filename: &str, credentials: &str) -> Result<String> {
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