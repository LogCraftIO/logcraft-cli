// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use console::Style;
use once_cell::sync::Lazy;
use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use std::{
    collections::BTreeSet,
    io::{self, BufWriter, StdoutLock, Write},
};

/// Global style definitions using once_cell.
pub static MODIFY_STYLE: Lazy<Style> = Lazy::new(|| Style::new().yellow());
pub static ADD_STYLE: Lazy<Style> = Lazy::new(|| Style::new().green());
pub static REMOVE_STYLE: Lazy<Style> = Lazy::new(|| Style::new().red());
pub static BOLD_STYLE: Lazy<Style> = Lazy::new(|| Style::new().bold());

/// Configuration for diff output.
#[derive(Debug, Clone)]
pub struct DiffConfig {
    /// Global indentation
    pub tab_size: usize,
    /// Indentation for multi-line blocks
    pub multiline_indent: usize,
}

impl Default for DiffConfig {
    fn default() -> Self {
        DiffConfig {
            tab_size: 3,
            multiline_indent: 3,
        }
    }
}

fn is_empty_value(val: &Value) -> bool {
    match val {
        Value::String(s) => s.trim().is_empty(),
        Value::Array(arr) => arr.is_empty(),
        Value::Object(obj) => obj.is_empty(),
        _ => false,
    }
}

/// Normalize a multi-line string by trimming each line and ensuring it ends with a newline.
#[inline]
fn normalize_multiline(text: &str) -> String {
    let mut result = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Internal recursive diff function.
/// The `path` parameter accumulates the field global path in the JSON document.
fn print_json_diff_internal<W: Write>(
    path: &str,
    desired: &Value,
    current: &Value,
    writer: &mut W,
    config: &DiffConfig,
) -> io::Result<()> {
    // Global prefix: spaces repeated tab_size times.
    let global_prefix = " ".repeat(config.tab_size);
    let tab_size = config.tab_size;
    let text_indent = config.multiline_indent;

    // Print empty values as additions.
    if is_empty_value(current) && !desired.is_null() {
        writeln!(
            writer,
            "{:<tab_size$}{}: {}",
            "",
            ADD_STYLE.apply_to(path),
            ADD_STYLE.apply_to(desired)
        )?;
        return Ok(());
    }

    match (desired, current) {
        // Objects.
        (Value::Object(d_obj), Value::Object(c_obj)) => {
            let keys: BTreeSet<_> = d_obj.keys().chain(c_obj.keys()).collect();
            for key in keys {
                let new_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", path, key)
                };
                match (d_obj.get(key), c_obj.get(key)) {
                    (Some(d_val), Some(c_val)) => {
                        print_json_diff_internal(&new_path, d_val, c_val, writer, config)?;
                    }
                    (Some(d_val), None) => {
                        writeln!(
                            writer,
                            "{:<tab_size$}{}: {}",
                            "",
                            ADD_STYLE.apply_to(&new_path),
                            ADD_STYLE.apply_to(d_val)
                        )?;
                    }
                    (None, Some(c_val)) => {
                        writeln!(
                            writer,
                            "{:<tab_size$}{}: {}",
                            "",
                            REMOVE_STYLE.apply_to(&new_path),
                            REMOVE_STYLE.apply_to(c_val)
                        )?;
                    }
                    _ => {}
                }
            }
        }
        // Arrays.
        (Value::Array(_), Value::Array(_)) => {
            if desired != current {
                writeln!(
                    writer,
                    "{:<tab_size$}{}: {} => {}",
                    "",
                    MODIFY_STYLE.apply_to(path),
                    REMOVE_STYLE.apply_to(current),
                    ADD_STYLE.apply_to(desired)
                )?;
            }
        }
        // Multi-line strings.
        (Value::String(d_str), Value::String(c_str))
            if d_str.contains('\n') || c_str.contains('\n') =>
        {
            let d_norm = normalize_multiline(d_str);
            let c_norm = normalize_multiline(c_str);
            if d_norm != c_norm {
                // Only print the field label if there is an actual diff.
                writeln!(writer, "{}{}: ", global_prefix, MODIFY_STYLE.apply_to(path))?;
                let diff = TextDiff::from_lines(&c_norm, &d_norm);
                for change in diff.iter_all_changes() {
                    match change.tag() {
                        ChangeTag::Delete => write!(
                            writer,
                            "{:<text_indent$}{:<tab_size$}{}",
                            "",
                            "",
                            REMOVE_STYLE.apply_to(format!("- {}", change)),
                        )?,
                        ChangeTag::Insert => write!(
                            writer,
                            "{:<text_indent$}{:<tab_size$}{}",
                            "",
                            "",
                            ADD_STYLE.apply_to(format!("+ {}", change)),
                        )?,
                        ChangeTag::Equal => write!(
                            writer,
                            "{:<text_indent$}{:<tab_size$}{}",
                            "",
                            "",
                            Style::new().dim().apply_to(change),
                            tab_size = tab_size + 2
                        )?,
                    }
                }
            }
            // If the normalized multi-line strings are identical, nothing is printed.
        }
        // Strings.
        (Value::String(d_str), Value::String(c_str)) => {
            if d_str != c_str {
                writeln!(
                    writer,
                    "{:<text_indent$}{}: {} => {}",
                    "",
                    MODIFY_STYLE.apply_to(path),
                    REMOVE_STYLE.apply_to(c_str),
                    ADD_STYLE.apply_to(d_str)
                )?;
            }
        }
        // All other types.
        _ => {
            if desired != current {
                writeln!(
                    writer,
                    "{:<text_indent$}{}: {} => {}",
                    "",
                    MODIFY_STYLE.apply_to(path),
                    REMOVE_STYLE.apply_to(current),
                    ADD_STYLE.apply_to(desired)
                )?;
            }
        }
    }

    Ok(())
}

/// Diff methods.
impl DiffConfig {
    /// Compare two JSON documents and return a formatted diff.
    pub fn diff_json(
        &self,
        desired: &Value,
        current: &Value,
        writer: &mut BufWriter<StdoutLock<'_>>,
    ) -> anyhow::Result<()> {
        // Start the recursive diff between the desired and current JSON values.
        writeln!(writer, "---")?;
        print_json_diff_internal("", desired, current, writer, self)?;
        writeln!(writer, "---")?;
        Ok(())
    }
}
