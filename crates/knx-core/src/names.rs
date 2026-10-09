//! Admission policy for new project-local names, never for imported source data.
use std::fmt;

pub const MAX_EDITED_NAME_CODEPOINTS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    Blank,
    TooLong,
    ControlCharacter,
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Blank => "name must not be empty or whitespace-only",
            Self::TooLong => "name must contain at most 1024 Unicode code points",
            Self::ControlCharacter => "name must not contain control characters or line separators",
        })
    }
}
impl std::error::Error for NameError {}

pub fn validate_edited_name(name: &str) -> Result<(), NameError> {
    if name.trim().is_empty() {
        return Err(NameError::Blank);
    }
    if name.chars().count() > MAX_EDITED_NAME_CODEPOINTS {
        return Err(NameError::TooLong);
    }
    if name
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err(NameError::ControlCharacter);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admission_counts_unicode_scalars_and_preserves_exact_text() {
        for name in ["  Küche 🛠  ", "e\u{301}", "重复", "<script>&literal"] {
            assert_eq!(validate_edited_name(name), Ok(()));
        }
        assert_eq!(validate_edited_name(&"🛠".repeat(1024)), Ok(()));
        assert_eq!(
            validate_edited_name(&"🛠".repeat(1025)),
            Err(NameError::TooLong)
        );
        for name in ["", " ", "\u{85}\u{2003}"] {
            assert_eq!(validate_edited_name(name), Err(NameError::Blank));
        }
        for c in [
            '\0', '\t', '\r', '\n', '\u{7f}', '\u{9f}', '\u{2028}', '\u{2029}',
        ] {
            assert_eq!(
                validate_edited_name(&format!("name{c}")),
                Err(NameError::ControlCharacter)
            );
        }
        // FEFF is not Unicode White_Space. No JavaScript trim-based normalization.
        assert_eq!(validate_edited_name("\u{feff}"), Ok(()));
    }
}
