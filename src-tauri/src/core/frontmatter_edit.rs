//! Byte-preserving edits to the two invocation frontmatter keys.
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::skill_discovery::{parse_invocation_mode, InvocationMode};

const KEYS: [&str; 2] = ["disable-model-invocation:", "user-invocable:"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvocationLines {
    pub disable_model_invocation: Option<String>,
    pub user_invocable: Option<String>,
    // Distinguishes an upstream empty block from one created by this writer.
    pub had_frontmatter: bool,
    // Invalid YAML can repeat keys; preserve even those bytes on clear.
    pub repeated_lines: Vec<(usize, String)>,
}

impl InvocationLines {
    pub fn mode(&self) -> InvocationMode {
        let mut text = format!(
            "---\n{}{}",
            self.disable_model_invocation.as_deref().unwrap_or(""),
            self.user_invocable.as_deref().unwrap_or("")
        );
        for (_, line) in &self.repeated_lines {
            text.push_str(line);
        }
        text.push_str("---\n");
        parse_invocation_mode(&text)
    }

    fn line(&self, key: usize) -> Option<&str> {
        if key == 0 {
            self.disable_model_invocation.as_deref()
        } else {
            self.user_invocable.as_deref()
        }
    }
}

fn header_end(lines: &[&str]) -> Option<usize> {
    if lines.first()?.trim() != "---" {
        return None;
    }
    lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim() == "---")
        .map(|(i, _)| i)
}

fn key_index(line: &str) -> Option<usize> {
    KEYS.iter().position(|key| line.starts_with(key))
}

fn ending(line: &str) -> &str {
    if line.ends_with("\r\n") {
        "\r\n"
    } else if line.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

pub fn read_invocation_lines(text: &str) -> InvocationLines {
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let end = header_end(&lines);
    let mut found: [Option<String>; 2] = [None, None];
    let mut repeated_lines = Vec::new();
    if let Some(end) = end {
        for (i, line) in lines.iter().enumerate().take(end).skip(1) {
            if let Some(key) = key_index(line) {
                if found[key].is_some() {
                    repeated_lines.push((i, (*line).to_string()));
                } else {
                    found[key] = Some((*line).to_string());
                }
            }
        }
    }
    InvocationLines {
        disable_model_invocation: found[0].take(),
        user_invocable: found[1].take(),
        had_frontmatter: end.is_some(),
        repeated_lines,
    }
}

pub fn write_invocation_mode(text: &str, mode: InvocationMode) -> String {
    if parse_invocation_mode(text) == mode {
        return text.to_string();
    }
    let values = [
        matches!(mode, InvocationMode::UserOnly | InvocationMode::Neither),
        matches!(
            mode,
            InvocationMode::UserAndModel | InvocationMode::UserOnly
        ),
    ];
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let Some(end) = header_end(&lines) else {
        return format!(
            "---\ndisable-model-invocation: {}\nuser-invocable: {}\n---\n{text}",
            values[0], values[1]
        );
    };
    let mut result = lines[0].to_string();
    let mut found = [false; 2];
    for line in &lines[1..end] {
        if let Some(key) = key_index(line) {
            found[key] = true;
            result.push_str(&format!("{} {}{}", KEYS[key], values[key], ending(line)));
        } else {
            result.push_str(line);
        }
    }
    for key in 0..2 {
        if !found[key] {
            result.push_str(&format!(
                "{} {}{}",
                KEYS[key],
                values[key],
                ending(lines[0])
            ));
        }
    }
    result.extend(lines[end..].iter().copied());
    result
}

pub fn restore_invocation_lines(text: &str, base: &InvocationLines) -> String {
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let Some(end) = header_end(&lines) else {
        return text.to_string();
    };
    if !base.had_frontmatter {
        return lines[end + 1..].concat();
    }
    let mut result = lines[0].to_string();
    let mut found = [false; 2];
    for (i, line) in lines.iter().enumerate().take(end).skip(1) {
        if let Some(key) = key_index(line) {
            if !found[key] {
                result.push_str(base.line(key).unwrap_or(""));
                found[key] = true;
            } else if let Some((_, original)) =
                base.repeated_lines.iter().find(|(pos, _)| *pos == i)
            {
                result.push_str(original);
            }
        } else {
            result.push_str(line);
        }
    }
    result.extend(lines[end..].iter().copied());
    result
}

/// Thin filesystem adapter; callers own mutation serialization and settlement.
pub fn apply_to_file(path: &Path, edit: impl FnOnce(&str) -> String) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let edited = edit(&text);
    if edited != text {
        std::fs::write(path, edited).with_context(|| format!("write {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/frontmatter_edit.rs"]
mod tests;
