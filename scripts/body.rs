use log::info;
use serde_json;
use std::fmt;
use std::str;

/// Formatea un body para su visualización en los logs
fn format_body_for_logging(body: &[u8], hide_body: bool, prefix: &str) {
    if !hide_body {
        // Check if body is actually empty
        let body_size = body.len();

        if body_size > 0 {
            // Log the body size
            info!("{} size: {} bytes", prefix, body_size);

            // Try to parse as JSON for pretty printing
            match String::from_utf8(body.to_vec()) {
                Ok(body_str) => {
                    if !body_str.trim().is_empty() {
                        match serde_json::from_str::<serde_json::Value>(&body_str) {
                            Ok(json_value) => {
                                // If it's valid JSON, beautify it
                                match serde_json::to_string_pretty(&json_value) {
                                    Ok(pretty) => info!("{}:\n{}", prefix, pretty),
                                    Err(_) => info!("{}:\n{}", prefix, body_str),
                                }
                            }
                            Err(_) => {
                                // Not valid JSON, use as-is
                                info!("{}:\n{}", prefix, body_str)
                            }
                        }
                    } else {
                        // Body converted to string is empty (whitespace only)
                        info!("{}: [empty string]", prefix);
                    }
                }
                Err(_) => {
                    // Not valid UTF-8, show length only
                    info!("{}: [binary data]", prefix);
                }
            }
        } else {
            info!("{}: [empty] (0 bytes)", prefix);
        }
    } else {
        info!("{}: [hidden] ({} bytes)", prefix, body.len());
    }
}
