use regex::Regex;
use std::vec::Vec;

const MARKDOWN_LINK_RE: &str = r"\[.*?\]\(.*?\)";
const UIUA_PAD_LINK_RE: &str = r"https?://(www\.)?uiua\.org/pad\?src=[0-9a-zA-Z_\-=]+";

fn strip_markdown_links(message: &str) -> String {
    Regex::new(MARKDOWN_LINK_RE)
        .expect("Failed to compile markdown link regex")
        .replace_all(message, "")
        .to_string()
}

/// Get a vector of all Uiua pad links not contained in markdown links
pub fn extract_raw_pad_link(message: &str) -> Vec<String> {
    Regex::new(UIUA_PAD_LINK_RE)
        .expect("Failed to compile uiua pad link regex")
        .find_iter(&strip_markdown_links(message))
        .map(|m| m.as_str().to_string())
        .collect()
}

/// Check if a message contains Uiua pad links not contained in markdown links
pub fn has_raw_pad_link(message: &str) -> bool {
    !extract_raw_pad_link(message).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTTP_WWW_RAW_PAD_LINK: &str = "http://www.uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg==";
    const HTTPS_WWW_RAW_PAD_LINK: &str = "https://www.uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg==";
    const HTTP_RAW_PAD_LINK: &str = "http://uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg==";
    const HTTPS_RAW_PAD_LINK: &str = "https://uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg==";
    const MD_PAD_LINK: &str = "[uiua](https://www.uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg==)";
    const MD_LINK: &str = "[google](https://google.com)";

    #[test]
    fn base_link_allowed() {
        assert!(extract_raw_pad_link("http://www.uiua.org/pad").is_empty());
        assert!(extract_raw_pad_link("https://www.uiua.org/pad").is_empty());
        assert!(extract_raw_pad_link("http://uiua.org/pad").is_empty());
        assert!(extract_raw_pad_link("https://uiua.org/pad").is_empty());
    }

    #[test]
    fn raw_link_disallowed() {
        assert_eq!(
            extract_raw_pad_link(HTTP_WWW_RAW_PAD_LINK),
            vec![HTTP_WWW_RAW_PAD_LINK],
        );
        assert_eq!(
            extract_raw_pad_link(HTTPS_WWW_RAW_PAD_LINK),
            vec![HTTPS_WWW_RAW_PAD_LINK],
        );
        assert_eq!(
            extract_raw_pad_link(HTTP_RAW_PAD_LINK),
            vec![HTTP_RAW_PAD_LINK],
        );
        assert_eq!(
            extract_raw_pad_link(HTTPS_RAW_PAD_LINK),
            vec![HTTPS_RAW_PAD_LINK],
        );
        assert_eq!(
            extract_raw_pad_link(format!("{HTTP_WWW_RAW_PAD_LINK} {HTTPS_WWW_RAW_PAD_LINK} {HTTP_RAW_PAD_LINK} {HTTPS_RAW_PAD_LINK}").as_str()),
            vec![HTTP_WWW_RAW_PAD_LINK, HTTPS_WWW_RAW_PAD_LINK, HTTP_RAW_PAD_LINK, HTTPS_RAW_PAD_LINK],
        );
    }

    #[test]
    fn markdown_link_allowed() {
        assert_eq!(extract_raw_pad_link(MD_PAD_LINK).len(), 0);
    }

    #[test]
    fn strips_markdown_link() {
        assert_eq!(strip_markdown_links(MD_PAD_LINK), "");
    }

    #[test]
    fn multiple_markdown_links() {
        assert_eq!(
            extract_raw_pad_link(format!("{MD_PAD_LINK} and some stuff {MD_LINK}").as_str()).len(),
            0,
        );
    }

    #[test]
    fn strips_markdown_links() {
        assert_eq!(
            strip_markdown_links(format!("{MD_PAD_LINK} and some stuff {MD_LINK}").as_str()),
            " and some stuff ",
        );
    }

    #[test]
    fn both() {
        assert_eq!(
            extract_raw_pad_link(
                format!("{HTTPS_WWW_RAW_PAD_LINK} and some stuff {MD_PAD_LINK}").as_str()
            ),
            vec![HTTPS_WWW_RAW_PAD_LINK],
        );
        assert_eq!(
            extract_raw_pad_link(
                format!("{HTTPS_WWW_RAW_PAD_LINK} and some stuff {MD_PAD_LINK} {HTTPS_WWW_RAW_PAD_LINK} and some more stuff {MD_PAD_LINK}")
                    .as_str()
            ),
            vec![HTTPS_WWW_RAW_PAD_LINK, HTTPS_WWW_RAW_PAD_LINK]
        );
    }

    #[test]
    fn strips_with_both() {
        assert_eq!(
            strip_markdown_links(
                format!("{HTTPS_WWW_RAW_PAD_LINK} and some stuff {MD_PAD_LINK}").as_str()
            ),
            format!("{HTTPS_WWW_RAW_PAD_LINK} and some stuff "),
        );
        assert_eq!(strip_markdown_links(
                format!("{HTTPS_WWW_RAW_PAD_LINK} and some stuff {MD_PAD_LINK} {HTTPS_WWW_RAW_PAD_LINK} and some more stuff {MD_PAD_LINK}")
                    .as_str()
            ),
            format!("{HTTPS_WWW_RAW_PAD_LINK} and some stuff  {HTTPS_WWW_RAW_PAD_LINK} and some more stuff ")
        );
        // "https://www.uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg=="
        assert_eq!(
            extract_raw_pad_link("https://uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg=="),
            vec!["https://uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg=="]
        );
    }
}
