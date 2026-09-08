//! Byte-preserving edits to the two invocation frontmatter keys.
use std::{io::Write, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::skill_discovery::{parse_invocation_mode, InvocationMode};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    DisableModelInvocation,
    UserInvocable,
}

impl Key {
    const ALL: [Self; 2] = [Self::DisableModelInvocation, Self::UserInvocable];

    fn label(self) -> &'static str {
        match self {
            Self::DisableModelInvocation => "disable-model-invocation:",
            Self::UserInvocable => "user-invocable:",
        }
    }

    fn value(self, mode: InvocationMode) -> bool {
        match self {
            Self::DisableModelInvocation => {
                matches!(mode, InvocationMode::UserOnly | InvocationMode::Neither)
            }
            Self::UserInvocable => matches!(
                mode,
                InvocationMode::UserAndModel | InvocationMode::UserOnly
            ),
        }
    }
}

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

    fn line(&self, key: Key) -> Option<&str> {
        match key {
            Key::DisableModelInvocation => self.disable_model_invocation.as_deref(),
            Key::UserInvocable => self.user_invocable.as_deref(),
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

fn line_key(line: &str) -> Option<Key> {
    Key::ALL
        .into_iter()
        .find(|key| line.starts_with(key.label()))
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
    let mut base = InvocationLines {
        disable_model_invocation: None,
        user_invocable: None,
        had_frontmatter: end.is_some(),
        repeated_lines: Vec::new(),
    };
    if let Some(end) = end {
        for (i, line) in lines.iter().enumerate().take(end).skip(1) {
            if let Some(key) = line_key(line) {
                let slot = match key {
                    Key::DisableModelInvocation => &mut base.disable_model_invocation,
                    Key::UserInvocable => &mut base.user_invocable,
                };
                if slot.is_some() {
                    base.repeated_lines.push((i, (*line).to_string()));
                } else {
                    *slot = Some((*line).to_string());
                }
            }
        }
    }
    base
}

pub fn write_invocation_mode(text: &str, mode: InvocationMode) -> String {
    if parse_invocation_mode(text) == mode {
        return text.to_string();
    }
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let Some(end) = header_end(&lines) else {
        return format!(
            "---\ndisable-model-invocation: {}\nuser-invocable: {}\n---\n{text}",
            Key::DisableModelInvocation.value(mode),
            Key::UserInvocable.value(mode)
        );
    };
    let mut result = lines[0].to_string();
    let mut found = Vec::new();
    for line in &lines[1..end] {
        if let Some(key) = line_key(line) {
            found.push(key);
            result.push_str(&format!(
                "{} {}{}",
                key.label(),
                key.value(mode),
                ending(line)
            ));
        } else {
            result.push_str(line);
        }
    }
    for key in Key::ALL {
        if !found.contains(&key) {
            result.push_str(&format!(
                "{} {}{}",
                key.label(),
                key.value(mode),
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
    if !base.had_frontmatter && lines[1..end].iter().all(|line| line_key(line).is_some()) {
        return lines[end + 1..].concat();
    }
    let mut result = lines[0].to_string();
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate().take(end).skip(1) {
        if let Some(key) = line_key(line) {
            if !found.contains(&key) {
                result.push_str(base.line(key).unwrap_or(""));
                found.push(key);
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
    let text = read_manifest(path)?;
    let edited = edit(&text);
    if edited != text {
        let temp_path =
            path.with_file_name(format!(".skills-hub-manifest-{}", uuid::Uuid::new_v4()));
        let mut temp = manifest_io(
            path,
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path),
        )?;
        let result = (|| -> std::io::Result<()> {
            temp.set_permissions(std::fs::metadata(path)?.permissions())?;
            temp.write_all(edited.as_bytes())?;
            temp.sync_all()?;
            drop(temp);
            std::fs::rename(&temp_path, path)
        })();
        if result.is_err() {
            // A crash can leave this hidden sibling; discovery and hashing ignore it.
            let _ = std::fs::remove_file(&temp_path);
        }
        manifest_io(path, result)?;
    }
    Ok(())
}

fn manifest_io<T>(path: &Path, result: std::io::Result<T>) -> Result<T> {
    result.map_err(|error| {
        let signal = super::errors::SignalError::SkillManifestIo {
            path: path.to_string_lossy().into_owned(),
            detail: error.to_string(),
        };
        anyhow::Error::new(error).context(signal)
    })
}

pub(crate) fn read_manifest(path: &Path) -> Result<String> {
    manifest_io(path, std::fs::read_to_string(path))
}

#[cfg(test)]
#[path = "tests/frontmatter_edit.rs"]
mod tests;
