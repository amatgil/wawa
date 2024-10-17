// import regex
use regex::Regex;

const MARKDOWN_LINK_RE: &str = r"\[.*?\]\(.*?\)";
const UIUA_PAD_LINK_PREFIX_HTTP: &str = "http://www.uiua.org/pad?src";
const UIUA_PAD_LINK_PREFIX_HTTPS: &str = "https://www.uiua.org/pad?src";

fn strip_markdown_links(message: &str) -> String {
    Regex::new(MARKDOWN_LINK_RE)
        .unwrap()
        .replace_all(message, "")
        .to_string()
}

pub fn has_raw_pad_link(message: &str) -> bool {
    // Remove all whitespace
    let message_no_whitespace: String = message.chars().filter(|c| !c.is_whitespace()).collect();
    let cleaned = strip_markdown_links(message_no_whitespace.as_str());
    cleaned.contains(UIUA_PAD_LINK_PREFIX_HTTP) || cleaned.contains(UIUA_PAD_LINK_PREFIX_HTTPS)
}
#[cfg(test)]
mod tests {
    // Bring the functions under test into scope
    use super::*;

    const RAW_PAD_LINK: &str = "https://www.uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg==";
    const MD_PAD_LINK: &str = "[uiua](https://www.uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg==)";

    #[test]
    fn base_link_allowed() {
        assert!(!has_raw_pad_link("http://www.uiua.org/pad"));
        assert!(!has_raw_pad_link("https://www.uiua.org/pad"));
    }

    #[test]
    fn raw_link_disallowed() {
        assert!(has_raw_pad_link(RAW_PAD_LINK));
        assert!(has_raw_pad_link(
            "http://www.uiua.org/pad?src=0_13_0-rc_4__4o2c4oqaCg=="
        ));
        assert!(has_raw_pad_link(UIUA_PAD_LINK_PREFIX_HTTP));
        assert!(has_raw_pad_link(UIUA_PAD_LINK_PREFIX_HTTPS));
    }

    #[test]
    fn markdown_link_allowed() {
        assert!(!has_raw_pad_link(MD_PAD_LINK));
    }

    #[test]
    fn strips_markdown_link() {
        assert_eq!(strip_markdown_links(MD_PAD_LINK), "");
    }

    #[test]
    fn multiple_markdown_links() {
        assert!(!has_raw_pad_link(
            format!("{MD_PAD_LINK} and some stuff [google](https://google.com)").as_str()
        ));
    }

    #[test]
    fn strips_markdown_links() {
        assert_eq!(
            strip_markdown_links(
                format!("{MD_PAD_LINK} and some stuff [google](https://google.com)").as_str()
            ),
            " and some stuff "
        );
    }

    #[test]
    fn both() {
        assert!(has_raw_pad_link(
            format!("{RAW_PAD_LINK} and some stuff {MD_PAD_LINK}").as_str()
        ));
        assert!(has_raw_pad_link(
            format!("{RAW_PAD_LINK} and some stuff {MD_PAD_LINK} {RAW_PAD_LINK} and some stuff {MD_PAD_LINK}").as_str()
        ));
    }

    #[test]
    fn strips_with_both() {
        assert_eq!(
            strip_markdown_links(format!("{RAW_PAD_LINK} and some stuff {MD_PAD_LINK}",).as_str(),),
            format!("{RAW_PAD_LINK} and some stuff ")
        );
        assert_eq!(strip_markdown_links(
                format!("{RAW_PAD_LINK} and some stuff {MD_PAD_LINK} {RAW_PAD_LINK} and some stuff {MD_PAD_LINK}").as_str(),
            ),
            format!("{RAW_PAD_LINK} and some stuff  {RAW_PAD_LINK} and some stuff "),
        );
    }
}
