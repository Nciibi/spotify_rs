//! Removes advertising and sponsored content from Web API responses.
//!
//! The web host blocks ad requests with resolver rules and page scripts. The
//! native front end has no browser, so it filters the data itself: Spotify
//! sometimes mixes objects whose `type` is `ad`, whose URI is a `spotify:ad:`
//! link, or which carry an explicit sponsored flag into the same paged lists
//! as real music.
use serde_json::Value;

/// Strip ad objects and ad-only fields from a decoded API response.
///
/// Returns the number of items/collections removed.
pub fn strip_ads(value: &mut Value) -> usize {
    fn walk(value: &mut Value, removed: &mut usize) {
        match value {
            Value::Array(items) => {
                items.retain(|item| !is_ad(item));
                for item in items.iter_mut() {
                    walk(item, removed);
                }
            }
            Value::Object(map) => {
                for key in ["ads", "advertisements", "sponsored_content", "ad"] {
                    if map.remove(key).is_some() {
                        *removed += 1;
                    }
                }
                for child in map.values_mut() {
                    walk(child, removed);
                }
            }
            _ => {}
        }
    }

    let mut removed = 0;
    walk(value, &mut removed);
    removed
}

fn is_ad(item: &Value) -> bool {
    if item["type"].as_str() == Some("ad") {
        return true;
    }
    if item["uri"].as_str().is_some_and(|uri| uri.starts_with("spotify:ad:")) {
        return true;
    }
    if item["is_sponsored"].as_bool() == Some(true) {
        return true;
    }
    matches!(item["name"].as_str(), Some(name)
        if name.trim().eq_ignore_ascii_case("advertisement")
            || name.trim().eq_ignore_ascii_case("sponsored"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_ads_from_paged_lists() {
        let mut value: Value = serde_json::from_str(
            r#"{"items":[
                {"type":"track","name":"Real song","uri":"spotify:track:1"},
                {"type":"ad","name":"Advertisement","uri":"spotify:ad:2"},
                {"type":"playlist","name":"Sponsored","uri":"spotify:playlist:3"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(strip_ads(&mut value), 2);
        assert_eq!(value["items"].as_array().unwrap().len(), 1);
        assert_eq!(value["items"][0]["name"], "Real song");
    }

    #[test]
    fn removes_ad_only_fields() {
        let mut value: Value =
            serde_json::from_str(r#"{"ads":[{"type":"ad"}],"items":[]}"#).unwrap();
        assert_eq!(strip_ads(&mut value), 1);
        assert!(value.get("ads").is_none());
    }
}
