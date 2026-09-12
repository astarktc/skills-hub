//! Manifest reads and byte-preserving invocation edits.
//! Discovery owns admission, Edit owns durable rows, and finalize owns settlement.
//! This module owns the grammar and required manifest I/O errors.
use std::{
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Find the actual SKILL.md file path in a directory (case-insensitive).
/// Returns the real filesystem path preserving original casing.
pub(crate) fn find_skill_md(dir: &Path) -> Option<PathBuf> {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name();
            if name.to_string_lossy().eq_ignore_ascii_case("skill.md") {
                return Some(path);
            }
        }
    }
    None
}

/// Find the closing column-zero fence, tolerating only trailing whitespace.
fn header_end(lines: &[&str]) -> Option<usize> {
    if lines.first()?.trim() != "---" {
        return None;
    }
    lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim_end() == "---")
        .map(|(i, _)| i)
}

/// Parse a SKILL.md's frontmatter into `(name, description)`; `None` if unusable.
pub(crate) fn parse_skill_md(path: &Path) -> Option<(String, Option<String>)> {
    parse_skill_md_with_reason(path).ok()
}

/// Parse a SKILL.md's frontmatter, reporting why it is unusable as a stable
/// token: `read_failed`, `invalid_frontmatter`, or `missing_name`.
pub(crate) fn parse_skill_md_with_reason(
    path: &Path,
) -> Result<(String, Option<String>), &'static str> {
    let text = read_text(path).map_err(|_| "read_failed")?;
    let lines: Vec<&str> = text.lines().collect();
    let end = header_end(&lines).ok_or("invalid_frontmatter")?;
    let mut name: Option<String> = None;
    let mut desc: Option<String> = None;
    let mut i = 1usize;
    while i < end {
        let raw = lines[i];
        let l = raw.trim();
        if let Some(v) = l.strip_prefix("name:") {
            name = Some(clean_frontmatter_value(v));
        } else if let Some(v) = l.strip_prefix("description:") {
            let v = v.trim();
            if v == "|" || v == ">" {
                let folded = v == ">";
                let mut block_lines: Vec<String> = Vec::new();
                while i + 1 < end {
                    let next = lines[i + 1];
                    if !next.trim().is_empty() && !next.starts_with(char::is_whitespace) {
                        break;
                    }
                    block_lines.push(next.strip_prefix("  ").unwrap_or(next).to_string());
                    i += 1;
                }
                let value = if folded {
                    block_lines
                        .iter()
                        .map(|line| line.trim())
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    block_lines.join("\n").trim().to_string()
                };
                desc = Some(value);
            } else {
                desc = Some(clean_frontmatter_value(v));
            }
        }
        i += 1;
    }
    let name = name.ok_or("missing_name")?;
    Ok((name, desc))
}

// ── Invocation mode ──

/// Who may invoke a skill, derived from its `SKILL.md` frontmatter.
///
/// Two Claude Code frontmatter keys govern this (the agentskills.io
/// specification does not define them yet):
/// `disable-model-invocation: true` blocks automatic model invocation, and
/// `user-invocable: false` hides the skill from the `/` menu. Both default to
/// the permissive value, so a skill with no frontmatter — or with malformed
/// frontmatter — is [`InvocationMode::UserAndModel`]. Setting both keys is the
/// documented recipe for hiding a skill from everyone: [`InvocationMode::Neither`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum InvocationMode {
    /// Default: the user can type `/name` and the model can load it on its own.
    #[default]
    UserAndModel,
    /// `disable-model-invocation: true` — only the user can invoke it.
    UserOnly,
    /// `user-invocable: false` — only the model can invoke it.
    ModelOnly,
    /// Both keys restrict invocation — neither the user nor the model can invoke it.
    Neither,
}

impl InvocationMode {
    const KEYS: [(Self, &'static str); 4] = [
        (Self::UserAndModel, "user-and-model"),
        (Self::UserOnly, "user-only"),
        (Self::ModelOnly, "model-only"),
        (Self::Neither, "neither"),
    ];

    pub fn as_key(self) -> &'static str {
        Self::KEYS.iter().find(|(mode, _)| *mode == self).unwrap().1
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::KEYS
            .iter()
            .find(|(_, value)| *value == key)
            .map(|(mode, _)| *mode)
    }
}

/// Invocation mode of the skill installed at `dir` (its `SKILL.md` is read
/// fresh). An unreadable or absent `SKILL.md` yields the default mode.
pub fn invocation_mode_for_dir(dir: &Path) -> InvocationMode {
    let Some(path) = find_skill_md(dir) else {
        return InvocationMode::default();
    };
    match read_text(&path) {
        Ok(text) => parse_invocation_mode(&text),
        Err(_) => InvocationMode::default(),
    }
}

/// Map a `SKILL.md`'s raw text to its [`InvocationMode`]. Never fails: any
/// shape that is not a recognised restriction means the default mode.
pub fn parse_invocation_mode(text: &str) -> InvocationMode {
    let lines: Vec<&str> = text.lines().collect();
    let Some(end) = header_end(&lines) else {
        return InvocationMode::default();
    };
    let mut model_disabled = false;
    let mut user_invocable = true;
    for raw in &lines[1..end] {
        let l = raw.trim();
        // Indented lines belong to a nested mapping (e.g. `metadata:`), not to
        // the top-level keys this reads.
        if raw.starts_with(char::is_whitespace) {
            continue;
        }
        if let Some(v) = l.strip_prefix("disable-model-invocation:") {
            if let Some(flag) = parse_frontmatter_bool(v) {
                model_disabled = flag;
            }
        } else if let Some(v) = l.strip_prefix("user-invocable:") {
            if let Some(flag) = parse_frontmatter_bool(v) {
                user_invocable = flag;
            }
        }
    }
    match (user_invocable, model_disabled) {
        (true, false) => InvocationMode::UserAndModel,
        (true, true) => InvocationMode::UserOnly,
        (false, false) => InvocationMode::ModelOnly,
        (false, true) => InvocationMode::Neither,
    }
}

/// A frontmatter boolean, accepting the spellings Claude Code accepts
/// (`true`/`false`, `yes`/`no`, `on`/`off`, `1`/`0`). `None` for anything else,
/// so a malformed value falls back to the key's default.
fn parse_frontmatter_bool(value: &str) -> Option<bool> {
    let value = clean_frontmatter_value(value).to_ascii_lowercase();
    match value.as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn clean_frontmatter_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

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

/// Required Edit read, including invalid UTF-8, raises `SkillManifestIo`.
pub(crate) fn read_manifest(path: &Path) -> Result<String> {
    manifest_io(path, read_text(path))
}

/// Shared text-read adapter for manifests and optional skill-lock enrichment.
/// The lock module owns JSON and provenance; it discards I/O failures rather
/// than surfacing a required-manifest error for an optional sidecar.
pub(crate) fn read_text(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

#[cfg(test)]
#[path = "tests/manifest.rs"]
mod tests;
