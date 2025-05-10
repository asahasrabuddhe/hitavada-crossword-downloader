use anyhow::Result;
use reqwest::{
    header::{HeaderMap, HeaderValue},
};

pub fn create_headers() -> Result<HeaderMap> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_headers() {
        let headers = create_headers().unwrap();
        
        // Test required headers are present
        assert!(headers.contains_key("accept"));
        assert!(headers.contains_key("accept-language"));
        assert!(headers.contains_key("content-type"));
        assert!(headers.contains_key("origin"));
        assert!(headers.contains_key("user-agent"));
        assert!(headers.contains_key("x-requested-with"));

        // Test header values
        assert_eq!(headers.get("accept").unwrap(), "*/*");
        assert_eq!(headers.get("origin").unwrap(), "https://www.ehitavada.com");
        assert_eq!(headers.get("content-type").unwrap(), "application/x-www-form-urlencoded; charset=UTF-8");
    }

    #[test]
    fn test_headers_are_valid() {
        let headers = create_headers().unwrap();
        
        // Test that all headers can be converted to strings
        for (name, value) in headers.iter() {
            assert!(!name.as_str().is_empty());
            assert!(!value.to_str().unwrap().is_empty());
        }
    }
} 