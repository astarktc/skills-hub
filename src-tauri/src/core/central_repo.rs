use std::path::Path;

use anyhow::{Context, Result};

use super::skill_store::SkillStore;
use super::sync_engine::copy_dir_recursive;

pub fn ensure_central_repo(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).with_context(|| format!("create {:?}", path))?;
    Ok(())
}

/// Relocate every managed skill directory into `new_base` (rename, falling
/// back to copy + delete across filesystems) and repoint its DB record.
/// Fails before touching anything if a source is missing or a target
/// already exists. Does not persist the new root — the settings policy does.
pub fn move_central_repo(store: &SkillStore, new_base: &Path) -> Result<()> {
    let skills = store.list_skills()?;
    let mut moves = Vec::with_capacity(skills.len());
    for skill in skills {
        let old_path = Path::new(&skill.central_path).to_path_buf();
        if !old_path.exists() {
            anyhow::bail!("central path not found: {:?}", old_path);
        }
        let file_name = old_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("invalid central path: {:?}", old_path))?;
        let new_path = new_base.join(file_name);
        if new_path.exists() {
            anyhow::bail!("target path already exists: {:?}", new_path);
        }
        moves.push((skill, old_path, new_path));
    }

    for (skill, old_path, new_path) in moves {
        if let Err(err) = std::fs::rename(&old_path, &new_path) {
            copy_dir_recursive(&old_path, &new_path)
                .with_context(|| format!("copy {:?} -> {:?}", old_path, new_path))?;
            std::fs::remove_dir_all(&old_path)
                .with_context(|| format!("cleanup {:?}", old_path))?;
            // Surface rename error in logs for troubleshooting.
            log::warn!("rename failed, fallback used: {}", err);
        }

        let mut updated = skill;
        updated.central_path = new_path.to_string_lossy().to_string();
        updated.updated_at = now_ms();
        store.upsert_skill(&updated)?;
    }
    Ok(())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
#[path = "tests/central_repo.rs"]
mod tests;
