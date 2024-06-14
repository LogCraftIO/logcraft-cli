// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use anyhow::{bail, Result};

pub fn ensure_kebab_case(name: &str) -> Result<String> {
    let s = name.to_ascii_lowercase();
    let mut chars = s.chars();

    // Validate the first character: it must be an alphanumeric lower-case character
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => (),
        _ => bail!(
            "bad format for name `{}`: must start with alphanumeric lower-case character",
            name
        ),
    }

    // Iterate through the rest of the characters
    while let Some(current) = chars.next() {
        if current == '-' {
            // Hyphen is allowed but should not be the last character
            // and the next character must be a valid alphanumeric character
            match chars.next() {
                // Continue
                Some(next) if next.is_ascii_lowercase() || next.is_ascii_digit() => {}
                // Last character is hyphen
                _ => bail!(
                    "bad format for name `{}`: must end with alphanumeric lower-case character",
                    name
                ),
            }
        }
        // Continue
        else if current.is_ascii_lowercase() || current.is_ascii_digit() {
        }
        // Invalid character found
        else {
            bail!(
                "bad format for name `{}`: must be and alphanumeric lower-case character or hyphen",
                name
            );
        }
    }

    Ok(s)
}
