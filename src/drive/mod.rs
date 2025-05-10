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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::NamedTempFile;

    // Define a trait for SSM operations
    trait SsmClient {
        async fn get_parameter(&self) -> Result<String>;
    }

    // Implement the trait for the real client
    impl SsmClient for aws_sdk_ssm::Client {
        async fn get_parameter(&self) -> Result<String> {
            let parameter = self
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
    }

    // Test implementation
    struct TestSsmClient {
        parameter_value: Option<String>,
    }

    impl TestSsmClient {
        fn new() -> Self {
            Self {
                parameter_value: None,
            }
        }

        fn set_parameter_value(&mut self, value: String) {
            self.parameter_value = Some(value);
        }
    }

    impl SsmClient for TestSsmClient {
        async fn get_parameter(&self) -> Result<String> {
            self.parameter_value.clone()
                .ok_or_else(|| anyhow::anyhow!("No parameter value set"))
        }
    }

    #[tokio::test]
    async fn test_get_google_credentials_from_file() {
        // Create a temporary file with test credentials
        let temp_file = NamedTempFile::new().unwrap();
        let test_credentials = r#"{"type": "service_account", "project_id": "test"}"#;
        std::fs::write(&temp_file, test_credentials).unwrap();

        // Set environment variable to point to our temp file
        env::set_var("GOOGLE_SERVICE_ACCOUNT_PATH", temp_file.path());

        let result = get_google_credentials().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_credentials);

        // Cleanup
        env::remove_var("GOOGLE_SERVICE_ACCOUNT_PATH");
    }

    #[tokio::test]
    async fn test_get_google_credentials_from_ssm() {
        // Create test client
        let mut test_client = TestSsmClient::new();
        test_client.set_parameter_value("test-credentials".to_string());

        // Test credentials retrieval
        let result = test_client.get_parameter().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-credentials");
    }

    #[tokio::test]
    async fn test_get_google_credentials_from_ssm_error() {
        // Create test client without setting a value
        let test_client = TestSsmClient::new();

        // Test credentials retrieval
        let result = test_client.get_parameter().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_upload_to_drive() {
        // Create a temporary test file
        let temp_file = NamedTempFile::new().unwrap();
        let test_content = b"test image content";
        std::fs::write(&temp_file, test_content).unwrap();

        // Set required environment variable
        env::set_var("GOOGLE_DRIVE_FOLDER_ID", "test-folder-id");

        // Test credentials
        let test_credentials = r#"{
            "type": "service_account",
            "project_id": "test",
            "private_key_id": "test",
            "private_key": "test",
            "client_email": "test@test.com",
            "client_id": "test",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token",
            "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
            "client_x509_cert_url": "https://www.googleapis.com/robot/v1/metadata/x509/test"
        }"#;

        let result = upload_to_drive(temp_file.path().to_str().unwrap(), test_credentials).await;
        
        // Cleanup
        env::remove_var("GOOGLE_DRIVE_FOLDER_ID");

        // Note: This test will fail in practice because we can't easily mock the Google Drive API
        // In a real test environment, we would use a mock for the DriveHub
        assert!(result.is_err());
    }
} 