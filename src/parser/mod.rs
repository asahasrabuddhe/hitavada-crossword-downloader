use scraper::{Html, Selector};
use crate::types::Rect;

/// Parses a single coords string into a Rect
pub fn parse_coords(coords_str: &str) -> Option<Rect> {
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
pub fn get_target_rect(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let area_selector = Selector::parse("area").unwrap();
    let tolerance_x1 = 5;
    let tolerance_y1 = 50;
    let tolerance_x2 = 10;
    let tolerance_y2 = 50;

    document.select(&area_selector)
        .find_map(|area| {
            if let Some(coords) = area.value().attr("coords") {
                if let Some(rect) = parse_coords(coords) {
                    // Check if coordinates are within tolerance
                    let x1_in_range = (rect.x1 - 0).abs() <= tolerance_x1;
                    let y1_in_range = (rect.y1 - 1625).abs() <= tolerance_y1;
                    let x2_in_range = (rect.x2 - 1000).abs() <= tolerance_x2;
                    let y2_in_range = (rect.y2 - 2775).abs() <= tolerance_y2;
                    
                    if x1_in_range && y1_in_range && x2_in_range && y2_in_range {
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
    fn test_get_target_rect_multiple_areas_with_tolerance() {
        let html = r#"
            <map>
                <area shape="rect" coords="0,89,1255,1683" href="test10">
                <area shape="rect" coords="1249,97,1749,1655" href="test11">
                <area shape="rect" coords="4,1672,997,2778" href="test12">
                <area shape="rect" coords="995,1664,1749,2778" href="test13">
            </map>
        "#;
        assert_eq!(get_target_rect(html), Some("test12".to_string()));
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