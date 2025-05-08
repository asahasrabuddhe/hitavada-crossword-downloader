#!/bin/bash
set -e

if [ -z "$GOOGLE_DRIVE_FOLDER_ID" ]; then
    echo "Error: GOOGLE_DRIVE_FOLDER_ID environment variable is not set"
    exit 1
fi

echo "Creating S3 bucket if it doesn't exist..."
aws s3api create-bucket \
    --bucket hitavada-crossword-deployments \
    --region ap-south-1 \
    --create-bucket-configuration LocationConstraint=ap-south-1 || true

echo "Building Lambda function..."
cargo lambda build --release

echo "Deploying with SAM..."
sam deploy --parameter-overrides "GoogleDriveFolderId=$GOOGLE_DRIVE_FOLDER_ID"

echo "Deployment complete!" 