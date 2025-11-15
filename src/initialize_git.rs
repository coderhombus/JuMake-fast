// src/initialize_gir.rs
//! Module responsible for initializing a Git repository for a JuMake project.
//! Handles `.gitignore`, adding all files, JUCE submodule linking, and initial commit.

use crate::context::Context;
use dialoguer::Input;
use dirs;
use git2::{Error as GitError, IndexAddOption, Repository, Signature};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[cfg(windows)]
use std::os::windows::fs as windows_fs;

// ------------------------
// Custom error type
// ------------------------
#[derive(Debug, Error)]
pub enum JuMakeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("TOML parse error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("Dialoguer error: {0}")]
    Dialoguer(#[from] dialoguer::Error),

    #[error("Symlink error from {src} to {dst}: {source}")]
    Symlink {
        src: String,
        dst: String,
        #[source]
        source: std::io::Error,
    },
}

// ------------------------
// Configuration handling
// ------------------------
#[derive(Serialize, Deserialize, Default)]
struct JuMakeConfig {
    juce_path: Option<PathBuf>,
}

/// Retrieves JUCE path from cached configuration or prompts the user.
/// Writes the path atomically to prevent config corruption.
pub fn get_juce_path() -> Result<PathBuf, JuMakeError> {
    // Platform-specific cache directory
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| JuMakeError::Config("Cannot determine cache directory".into()))?
        .join("jumake");

    fs::create_dir_all(&cache_dir)?;
    let config_file = cache_dir.join("config.toml");

    // Load or initialize config
    let mut config: JuMakeConfig = if config_file.exists() {
        toml::from_str(&fs::read_to_string(&config_file)?)?
    } else {
        JuMakeConfig::default()
    };

    // Prompt user if JUCE path is missing
    if config.juce_path.is_none() {
        let input_path: String = Input::new()
            .with_prompt("Enter path to your local JUCE folder")
            .validate_with(|input: &String| {
                let p = Path::new(input);
                if p.exists() && p.is_dir() {
                    Ok(())
                } else {
                    Err("Path does not exist or is not a directory")
                }
            })
            .interact_text()?;

        let pathbuf = PathBuf::from(&input_path);

        // Atomic write to avoid corrupt config
        let tmp_file = config_file.with_extension("tmp");
        fs::write(&tmp_file, toml::to_string(&JuMakeConfig { juce_path: Some(pathbuf.clone()) })?)?;
        fs::rename(&tmp_file, &config_file)?;

        config.juce_path = Some(pathbuf);
        info!("✅ JUCE path saved to {}", config_file.display());
    } else {
        info!("Using cached JUCE path from {}", config_file.display());
    }

    Ok(config.juce_path.expect("JUCE path must be set"))
}

// ------------------------
// Git repository initialization
// ------------------------
pub fn initialize_git_repo(context: &Context) -> Result<(), JuMakeError> {
    info!("Initializing Git repository at {}", context.project_path.display());

    let repo = Repository::init(&context.project_path)?;
    info!("Git repository initialized successfully.");

    append_gitignore(&context.project_path)?;
    add_all_files_to_repo(&repo)?;
    add_juce_submodule(context)?;

    stage_gitmodules_if_exists(&repo, &context.project_path)?;

    Ok(())
}

/// Stages `.gitmodules` if it exists
fn stage_gitmodules_if_exists(repo: &Repository, project_path: &Path) -> Result<(), JuMakeError> {
    let gitmodules_path = project_path.join(".gitmodules");

    if gitmodules_path.exists() {
        match repo.index().and_then(|mut idx| { 
            idx.add_path(Path::new(".gitmodules"))?;
            idx.write() 
        }) {
            Ok(_) => info!("✅ .gitmodules file successfully staged."),
            Err(e) => warn!("⚠️  .gitmodules exists but could not be staged: {}", e),
        }
    } else {
        info!("No .gitmodules file found — skipping stage step.");
    }

    Ok(())
}

// ------------------------
// .gitignore handling
// ------------------------
const DEFAULT_GITIGNORE: &[&str] = &[
    "modules/",
    "jumake_build/",
    "build/",
    "compile_commands.json",
    ".jumake",
    ".cache/",
];

fn append_gitignore(project_path: &Path) -> Result<(), JuMakeError> {
    let gitignore_path = project_path.join(".gitignore");
    let existing = fs::read_to_string(&gitignore_path).unwrap_or_default();

    let new_entries: String = DEFAULT_GITIGNORE
        .iter()
        .filter(|entry| !existing.contains(*entry))
        .map(|entry| format!("{}\n", entry))
        .collect();

    if !new_entries.is_empty() {
        let tmp_path = gitignore_path.with_extension("tmp");
        fs::write(&tmp_path, format!("{}{}", existing, new_entries))?;
        fs::rename(&tmp_path, &gitignore_path)?;
        info!("✅ Updated .gitignore at {}", gitignore_path.display());
    } else {
        info!("No new entries to add to .gitignore");
    }

    Ok(())
}

// ------------------------
// Add JUCE submodule (cross-platform symlink)
// ------------------------
fn add_juce_submodule(context: &Context) -> Result<(), JuMakeError> {
    let juce_path = get_juce_path()?;
    if !juce_path.is_dir() {
        return Err(JuMakeError::Config(format!(
            "Local JUCE folder does not exist: {}",
            juce_path.display()
        )));
    }

    let modules_path = context.project_path.join("modules");
    fs::create_dir_all(&modules_path)?;
    let juce_link = modules_path.join("JUCE");

    // Remove existing incorrect symlink or folder
    if juce_link.exists() {
        match fs::read_link(&juce_link) {
            Ok(existing_target) if existing_target == juce_path => {
                info!("JUCE symlink already correct: {} → {}", juce_link.display(), juce_path.display());
                return Ok(());
            }
            _ => {
                warn!("Replacing existing JUCE link/folder at {}", juce_link.display());
                if juce_link.is_dir() {
                    fs::remove_dir_all(&juce_link)?;
                } else {
                    fs::remove_file(&juce_link)?;
                }
            }
        }
    }

    info!("Creating symlink from {} → {}", juce_path.display(), juce_link.display());
    create_symlink(&juce_path, &juce_link)?;
    info!("✅ Linked JUCE to {}", juce_link.display());

    Ok(())
}

// ------------------------
// Cross-platform symlink creation
// ------------------------
#[cfg(unix)]
fn create_symlink(src: &Path, dst: &Path) -> Result<(), JuMakeError> {
    std::os::unix::fs::symlink(src, dst).map_err(|source| JuMakeError::Symlink {
        src: src.display().to_string(),
        dst: dst.display().to_string(),
        source,
    })
}

#[cfg(windows)]
fn create_symlink(src: &Path, dst: &Path) -> Result<(), JuMakeError> {
    windows_fs::symlink_dir(src, dst).map_err(|source| JuMakeError::Symlink {
        src: src.display().to_string(),
        dst: dst.display().to_string(),
        source,
    })
}

// ------------------------
// Add all files to Git index
// ------------------------
fn add_all_files_to_repo(repo: &Repository) -> Result<(), GitError> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

// ------------------------
// Initial commit
// ------------------------
pub fn create_initial_commit(context: &Context) -> Result<(), JuMakeError> {
    let repo = Repository::open(&context.project_path)?;
    let signature = Signature::now("JuMake", "jumake@example.com")?;

    let tree_id = repo.index()?.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let commit_id = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit by JuMake",
        &tree,
        &[],
    )?;

    info!("✅ Initial commit created (id: {})", commit_id);
    Ok(())
}
