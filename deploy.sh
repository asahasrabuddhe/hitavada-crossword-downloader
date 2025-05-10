#!/bin/bash
set -e

if [ -z "$GOOGLE_DRIVE_FOLDER_ID" ]; then
    echo "Error: GOOGLE_DRIVE_FOLDER_ID environment variable is not set"
    exit 1
fi

if [ -z "$GOOGLE_SERVICE_ACCOUNT_PATH" ]; then
    echo "Error: GOOGLE_SERVICE_ACCOUNT_PATH environment variable is not set"
    exit 1
fi

echo "Creating S3 bucket if it doesn't exist..."
aws s3api create-bucket \
    --profile personal \
    --bucket hitavada-crossword-sam-deployments \
    --region ap-south-1 \
    --create-bucket-configuration LocationConstraint=ap-south-1 || true

echo "Building Lambda function..."
cargo lambda build --release --target x86_64-unknown-linux-musl

echo "Reading Google service account JSON..."
GOOGLE_SERVICE_ACCOUNT_JSON=$(cat "$GOOGLE_SERVICE_ACCOUNT_PATH")

echo "Deploying with SAM..."
sam deploy \
    --parameter-overrides \
    "GoogleDriveFolderId=$GOOGLE_DRIVE_FOLDER_ID" \
    "GoogleServiceAccountJson=$GOOGLE_SERVICE_ACCOUNT_JSON" \
    --profile personal

echo "Deployment complete!" 