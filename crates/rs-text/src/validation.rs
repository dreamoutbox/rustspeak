use crate::TextError;

const MIN_CHANNEL_NAME_LEN: usize = 1;
const MAX_CHANNEL_NAME_LEN: usize = 32;
const MAX_TOPIC_LEN: usize = 200;
const MAX_MESSAGE_LEN: usize = 2000;

/// Validates and trims a text channel `name` (1..=32 ASCII `[a-z0-9_-]`).
pub fn validate_channel_name(name: &str) -> Result<String, TextError> {
    let normalized = name.trim().to_ascii_lowercase();
    if normalized.len() < MIN_CHANNEL_NAME_LEN || normalized.len() > MAX_CHANNEL_NAME_LEN {
        return Err(TextError::InvalidInput(format!(
            "channel name must be {MIN_CHANNEL_NAME_LEN}..={MAX_CHANNEL_NAME_LEN} characters"
        )));
    }
    let valid = normalized
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
    if !valid {
        return Err(TextError::InvalidInput(
            "channel name may only contain letters, digits, '-' and '_'".to_string(),
        ));
    }
    Ok(normalized)
}

/// Validates and trims a channel `topic` (up to 200 chars).
pub fn validate_topic(topic: &str) -> Result<&str, TextError> {
    let trimmed = topic.trim();
    if trimmed.chars().count() > MAX_TOPIC_LEN {
        return Err(TextError::InvalidInput(format!(
            "topic must be at most {MAX_TOPIC_LEN} characters"
        )));
    }
    Ok(trimmed)
}

/// Validates and trims message `content` (non-empty, up to 2000 chars).
pub fn validate_message_content(content: &str) -> Result<&str, TextError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(TextError::InvalidInput(
            "message cannot be empty".to_string(),
        ));
    }
    if trimmed.chars().count() > MAX_MESSAGE_LEN {
        return Err(TextError::InvalidInput(format!(
            "message must be at most {MAX_MESSAGE_LEN} characters"
        )));
    }
    Ok(trimmed)
}
