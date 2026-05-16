//! Slug format shared by the API and the CRM so both reject the same inputs.

/// A slug is a non-empty, lowercase, kebab token: ASCII `a-z`, `0-9` and
/// single internal hyphens — no leading/trailing hyphen, no `--`, no dots,
/// spaces, underscores or non-ASCII. Used for galleries and blog posts.
pub fn is_valid_slug(s: &str) -> bool {
    if s.is_empty() || s.starts_with('-') || s.ends_with('-') || s.contains("--") {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::is_valid_slug;

    #[test]
    fn accepts_kebab_tokens() {
        for s in ["japon", "a", "a1", "sesion-tokio", "2024-retratos", "x-y-z"] {
            assert!(is_valid_slug(s), "{s:?} should be valid");
        }
    }

    #[test]
    fn rejects_malformed() {
        for s in [
            "", "j.pn", "Japon", "a b", "a_b", "-a", "a-", "a--b", "café",
            "a/b", "Á", "UPPER", " ",
        ] {
            assert!(!is_valid_slug(s), "{s:?} should be invalid");
        }
    }
}
