//! Input validation to prevent command injection and other attacks.

use crate::{Result, VaultmuxError};

/// Dangerous characters that could enable command injection in shell commands.
const DANGEROUS_CHARS: &str = ";|&$`<>(){}[]!*?~#%^\\\"'";

/// Maximum allowed length for item/location names.
const MAX_NAME_LENGTH: usize = 255;

/// Validates an item name for safety.
///
/// This function prevents command injection attacks by checking for:
/// - Empty names
/// - Excessive length (>255 characters)
/// - Null bytes
/// - Control characters
/// - Shell metacharacters that could enable injection
///
/// # Errors
///
/// Returns [`VaultmuxError::InvalidItemName`] if validation fails.
///
/// # Example
///
/// ```
/// use vaultmux::validation::validate_item_name;
///
/// assert!(validate_item_name("my-api-key").is_ok());
/// assert!(validate_item_name("api_key_123").is_ok());
/// assert!(validate_item_name("prod.database.password").is_ok());
///
/// assert!(validate_item_name("").is_err());
/// assert!(validate_item_name("name; rm -rf /").is_err());
/// assert!(validate_item_name("name$(whoami)").is_err());
/// ```
pub fn validate_item_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(VaultmuxError::InvalidItemName(
            "name cannot be empty".to_string(),
        ));
    }

    if name.len() > MAX_NAME_LENGTH {
        return Err(VaultmuxError::InvalidItemName(format!(
            "name exceeds maximum length of {} characters",
            MAX_NAME_LENGTH
        )));
    }

    if name.contains('\0') {
        return Err(VaultmuxError::InvalidItemName(
            "name contains null byte".to_string(),
        ));
    }

    if name
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(VaultmuxError::InvalidItemName(
            "name contains control characters".to_string(),
        ));
    }

    if name.chars().any(|c| DANGEROUS_CHARS.contains(c)) {
        return Err(VaultmuxError::InvalidItemName(format!(
            "name contains dangerous characters (not allowed: {})",
            DANGEROUS_CHARS
        )));
    }

    Ok(())
}

/// Validates a location name for safety.
///
/// Uses the same rules as [`validate_item_name`].
pub fn validate_location_name(name: &str) -> Result<()> {
    validate_item_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_names() {
        assert!(validate_item_name("my-secret").is_ok());
        assert!(validate_item_name("API_KEY_123").is_ok());
        assert!(validate_item_name("prod.database.password").is_ok());
        assert!(validate_item_name("user@example.com").is_ok());
        assert!(validate_item_name("path/to/secret").is_ok());
    }

    #[test]
    fn test_empty_name() {
        let result = validate_item_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_too_long() {
        let long_name = "a".repeat(256);
        let result = validate_item_name(&long_name);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("maximum length"));
    }

    #[test]
    fn test_null_byte() {
        let result = validate_item_name("name\0with\0nulls");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("null byte"));
    }

    #[test]
    fn test_control_characters() {
        let result = validate_item_name("name\x01with\x02control");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("control"));
    }

    #[test]
    fn test_command_injection_attempts() {
        let dangerous_names = vec![
            "name; rm -rf /",
            "name|grep password",
            "name&&whoami",
            "name$(whoami)",
            "name`id`",
            "name<input>output",
            "name{a,b,c}",
            "name[0-9]",
            "name!dangerous",
            "name*wildcard",
            "name?question",
            "name~home",
            "name#comment",
            "name%percent",
            "name^caret",
            "name\\backslash",
            "name\"quote",
            "name'apostrophe",
        ];

        for name in dangerous_names {
            let result = validate_item_name(name);
            assert!(result.is_err(), "Expected '{}' to fail validation", name);
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("dangerous characters"));
        }
    }

    #[test]
    fn test_location_name_validation() {
        assert!(validate_location_name("work-secrets").is_ok());
        assert!(validate_location_name("folder; rm -rf /").is_err());
    }
}
