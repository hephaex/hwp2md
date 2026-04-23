pub(crate) const SAFE_URL_SCHEMES: &[&str] = &[
    "http://", "https://", "mailto:", "tel:", "ftp://", "ftps://",
];

pub(crate) fn is_safe_url_scheme(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    SAFE_URL_SCHEMES.iter().any(|s| lower.starts_with(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_safe_url_scheme_accepts_https() {
        assert!(is_safe_url_scheme("https://example.com"));
        assert!(is_safe_url_scheme("HTTP://EXAMPLE.COM"));
        assert!(is_safe_url_scheme("mailto:user@example.com"));
    }

    #[test]
    fn is_safe_url_scheme_accepts_tel() {
        assert!(is_safe_url_scheme("tel:+1-555-0100"));
    }

    #[test]
    fn is_safe_url_scheme_rejects_dangerous() {
        assert!(!is_safe_url_scheme("javascript:alert(1)"));
        assert!(!is_safe_url_scheme("data:text/html,<h1>hi</h1>"));
        assert!(!is_safe_url_scheme("vbscript:msgbox"));
    }
}
