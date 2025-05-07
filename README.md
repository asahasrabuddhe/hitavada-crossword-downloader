# Hitavada Crossword Downloader

A Rust program to download crosswords from The Hitavada e-paper.

## Setup

1. Clone this repository
2. Create a `config.toml` file in the root directory with your Hitavada e-paper credentials:
   ```toml
   [credentials]
   username = "your_username"
   password = "your_password"
   ```
3. Build the program:
   ```bash
   cargo build --release
   ```

## Usage

Simply run the program:
```bash
cargo run
```

The program will:
1. Log in to your Hitavada e-paper account
2. Navigate to today's crossword page
3. Find and print the download URL for the crossword

## Requirements

- Rust 1.56 or later
- A valid Hitavada e-paper subscription
- Internet connection

## Note

This program is for personal use only. Please respect The Hitavada's terms of service and do not distribute downloaded content. 