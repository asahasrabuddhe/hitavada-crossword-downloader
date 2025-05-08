# Hitavada Crossword Downloader - AWS Lambda

This is a Rust-based AWS Lambda function that downloads the daily crossword from ehitavada.com and uploads it to Google Drive.

## Prerequisites

- Rust and Cargo installed
- AWS CLI configured with appropriate credentials
- Docker installed (for building the Lambda deployment package)
- Google Cloud project with Drive API enabled
- Google service account with Drive API access
- AWS SAM CLI installed

## Environment Setup

1. Create a `.env` file in the project root with your credentials:
```bash
GOOGLE_DRIVE_FOLDER_ID=your_folder_id
GOOGLE_SERVICE_ACCOUNT_PATH=path/to/service-account.json
```

2. Store the Google service account JSON in AWS Secrets Manager:
```bash
aws secretsmanager create-secret \
    --name google-service-account \
    --description "Google service account credentials" \
    --secret-string "$(cat path/to/service-account.json)"
```

## Building and Deploying

1. Install the `cargo-lambda` tool:
```bash
cargo install cargo-lambda
```

2. Install the cross-compilation target:
```bash
rustup target add x86_64-unknown-linux-gnu
```

3. Build the Lambda function:
```bash
cargo lambda build --release
```

4. Deploy using SAM CLI:
```bash
# First time deployment
sam deploy --guided

# Subsequent deployments
sam deploy
```

During the guided deployment, you'll be asked for:
- Stack name
- AWS Region
- Google Drive folder ID
- Confirm changes before deploy
- Allow SAM CLI IAM role creation
- Save arguments to configuration file

## Lambda Configuration

The Lambda function is configured with:
- Runtime: Custom runtime (Rust)
- Architecture: x86_64
- Memory: 256 MB
- Timeout: 30 seconds
- Environment variables:
  - `GOOGLE_DRIVE_FOLDER_ID`: Your Google Drive folder ID
- IAM Role: Automatically configured with Secrets Manager access
- EventBridge Schedule: Runs daily at midnight UTC

## Invoking the Lambda

You can invoke the Lambda function with an optional date parameter:

```json
{
    "date": "2024-03-20"  // Optional, defaults to today's date
}
```

The function will return:
```json
{
    "message": "Crossword downloaded successfully",
    "filename": "/tmp/crossword_2024-03-20.jpg"
}
```

## Notes

- The function saves the crossword image to the `/tmp` directory, which is the only writable location in AWS Lambda
- The image will be automatically cleaned up when the Lambda execution environment is recycled
- Google service account credentials are securely stored in AWS Secrets Manager
- The function will upload the downloaded crossword to the specified Google Drive folder
- The function is automatically triggered daily via EventBridge

## Error Handling

The function includes proper error handling and logging. All errors are logged to CloudWatch Logs.

## Development

To test locally with SAM:
```bash
sam local invoke CrosswordDownloaderFunction --event events/event.json
```

To start a local API:
```bash
sam local start-api
``` 