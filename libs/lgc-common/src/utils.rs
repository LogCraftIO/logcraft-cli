// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use std::{collections::HashMap, path::PathBuf};

use anyhow::{bail, Result};

use crate::configuration::{DetectionContext, LGC_BASE_DIR};

/// Ensure that a string is in kebab-case format
pub fn ensure_kebab_case(name: String) -> Result<String> {
    let mut chars = name.chars();

    // Validate the first character must be alphanumeric (lowercase or digit).
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => (),
        _ => bail!("invalid format `{}`, must be kebab-case", name),
    }

    // Iterate over the remaining characters
    while let Some(current) = chars.next() {
        if current == '-' {
            // A hyphen cannot be the last character
            // and must be followed by a valid alphanumeric character.
            match chars.next() {
                Some(next) if next.is_ascii_lowercase() || next.is_ascii_digit() => (),
                // Either no character after the hyphen or it's invalid
                _ => bail!("invalid format `{}`, must be kebab-case", name),
            }
        } else if current.is_ascii_lowercase() || current.is_ascii_digit() {
            // Valid alphanumeric — continue checking
        } else {
            // Invalid character found
            bail!("invalid format `{}`, must be kebab-case", name);
        }
    }

    // If all checks pass, return the original String
    Ok(name)
}

/// Check if a string contains forbidden characters for environment variables
pub fn env_forbidden_chars(s: &str) -> bool {
    for c in s.chars() {
        if c == '$' || c == '{' || c == '}' {
            return true;
        }
    }
    false
}

/// Convert a string to kebab-case
pub fn to_kebab_case(input: &str) -> Result<String, &'static str> {
    // Check if input is empty
    if input.is_empty() {
        return Err("invalid input, must not be empty");
    }

    // Create a new string with the same capacity as the input
    let mut kebab = String::with_capacity(input.len());
    let mut prev_char_was_delimiter = true; // Avoids leading hyphen
    let chars = input.chars().peekable();

    for c in chars {
        if c.is_ascii_alphabetic() || c.is_ascii_digit() {
            if c.is_uppercase() {
                kebab.push(c.to_ascii_lowercase());
            } else {
                kebab.push(c);
            }
            prev_char_was_delimiter = false;
        } else if c == ' ' || c == '_' || c == '-' {
            // Replace delimiters with a single hyphen.
            if !prev_char_was_delimiter && !kebab.ends_with('-') {
                kebab.push('-');
                prev_char_was_delimiter = true;
            }
        } else {
            // For any other characters, you can choose to skip or handle them.
            // Here, we'll skip them.
            // Alternatively, you could replace them with a hyphen or remove them.
        }
    }

    // Check if formatted string is empty
    if kebab.is_empty() {
        return Err("invalid input, must have at least one alphanumeric character");
    }

    // Remove trailing hyphen if present.
    if kebab.ends_with('-') {
        kebab.pop();
    }

    Ok(kebab)
}

pub fn filter_missing_plugins<T>(
    base_dir: Option<String>,
    workspace: &str,
    context: &mut HashMap<String, T>,
) -> PathBuf
where
    T: AsRef<DetectionContext>,
{
    let plugins_dir = PathBuf::from(base_dir.as_deref().unwrap_or(LGC_BASE_DIR)).join("plugins");

    context.retain(|name, _| {
        let exists = plugins_dir.join(name).with_extension("wasm").exists();
        if !exists {
            tracing::warn!("ignoring '{}/{}' (no matching plugin).", workspace, name);
        }
        exists
    });

    plugins_dir
}
