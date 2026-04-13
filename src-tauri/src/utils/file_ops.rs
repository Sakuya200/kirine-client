use std::{
    fs,
    path::Path,
};

use anyhow::Context;

use crate::Result;

pub fn ensure_parent_dir(path: &Path, description: &str) -> Result<()> {
    if let Some(parent_dir) = path.parent() {
        fs::create_dir_all(parent_dir).with_context(|| {
            format!(
                "failed to create {} directory: {}",
                description,
                parent_dir.display()
            )
        })?;
    }

    Ok(())
}

pub fn ensure_directory(path: &Path, description: &str) -> Result<()> {
    fs::create_dir_all(path).with_context(|| {
        format!(
            "failed to create {} directory: {}",
            description,
            path.display()
        )
    })?;

    Ok(())
}

pub fn directory_has_entries(path: &Path, description: &str) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let mut entries = fs::read_dir(path).with_context(|| {
        format!(
            "failed to read {} directory: {}",
            description,
            path.display()
        )
    })?;

    Ok(entries
        .next()
        .transpose()
        .with_context(|| {
            format!(
                "failed to inspect {} directory: {}",
                description,
                path.display()
            )
        })?
        .is_some())
}

fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

fn copy_dir_recursive(source_path: &Path, target_path: &Path, description: &str) -> Result<()> {
    ensure_directory(target_path, description)?;

    for entry in fs::read_dir(source_path).with_context(|| {
        format!(
            "failed to read {} directory: {}",
            description,
            source_path.display()
        )
    })? {
        let entry = entry.with_context(|| {
            format!(
                "failed to enumerate {} directory: {}",
                description,
                source_path.display()
            )
        })?;
        let entry_path = entry.path();
        let destination_path = target_path.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &destination_path, description)?;
            continue;
        }

        fs::copy(&entry_path, &destination_path).with_context(|| {
            format!(
                "failed to copy {} file from {} to {}",
                description,
                entry_path.display(),
                destination_path.display()
            )
        })?;
    }

    Ok(())
}

pub fn migrate_directory(
    source_path: &Path,
    target_path: &Path,
    description: &str,
    remove_source_after_copy: bool,
) -> Result<bool> {
    if same_path(source_path, target_path) {
        ensure_directory(target_path, description)?;
        return Ok(false);
    }

    if !source_path.exists() || !directory_has_entries(source_path, description)? {
        ensure_directory(target_path, description)?;
        return Ok(false);
    }

    if target_path.exists() && directory_has_entries(target_path, description)? {
        anyhow::bail!(
            "目标{}目录已存在且非空，请先清理后再迁移: {}",
            description,
            target_path.display()
        );
    }

    if let Some(parent_dir) = target_path.parent() {
        ensure_directory(parent_dir, description)?;
    }

    copy_dir_recursive(source_path, target_path, description)?;

    if remove_source_after_copy {
        fs::remove_dir_all(source_path).with_context(|| {
            format!(
                "failed to remove old {} directory: {}",
                description,
                source_path.display()
            )
        })?;
    }

    Ok(true)
}

pub fn replace_output_file(
    source_path: &Path,
    target_path: &Path,
    description: &str,
) -> Result<()> {
    if source_path == target_path {
        return Ok(());
    }

    if target_path.exists() {
        fs::remove_file(target_path).with_context(|| {
            format!(
                "failed to replace existing {} file: {}",
                description,
                target_path.display()
            )
        })?;
    }

    fs::rename(source_path, target_path).with_context(|| {
        format!(
            "failed to move {} from {} to {}",
            description,
            source_path.display(),
            target_path.display()
        )
    })?;

    Ok(())
}

pub fn remove_file_if_exists(path: &Path, description: &str) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)
            .with_context(|| format!("failed to clean {}: {}", description, path.display()))?;
    }

    Ok(())
}

