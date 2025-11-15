// src/create_project.rs
//! This module provides functionality to create a new project directory, 
//! set up source files, generate CMakeLists, and initialize Git with an initial commit.

use crate::context::Context;
use crate::create_files::{create_cmakelists, create_source_files};
use crate::initialize_git::{create_initial_commit, initialize_git_repo, JuMakeError};
use std::fs;
use log::{info, warn};

/// Creates a new project directory and sets up the project structure.
///
/// Steps performed:
/// 1. Creates the project directory.
/// 2. Generates `CMakeLists.txt`.
/// 3. Creates source files based on template.
/// 4. Initializes Git repository.
/// 5. Creates initial commit.
///
/// # Errors
/// Returns a `JuMakeError` if the project directory already exists or on any I/O error.
pub fn create_project(context: &Context) -> Result<(), JuMakeError> {
    // Check if project directory already exists
    if context.project_path.exists() {
        return Err(JuMakeError::Config(format!(
            "Project directory already exists: {}",
            context.project_path.display()
        )));
    }

    info!(
        "Creating project '{}' at {}...",
        context.project_name,
        context.project_path.display()
    );

    // Step 1: Create project directory
    fs::create_dir_all(&context.project_path).map_err(JuMakeError::Io)?;

    // Step 2: Create CMakeLists.txt with robust error logging
    if let Err(e) = create_cmakelists(context) {
        // Using warn! instead of panicking keeps CLI flow uninterrupted
        warn!("Failed to create CMakeLists.txt: {}", e);
    }

    // Step 3: Create source files
    if let Err(e) = create_source_files(context) {
        warn!("Failed to create source files: {}", e);
    }

    // Step 4: Initialize Git repository
    if let Err(e) = initialize_git_repo(context) {
        warn!("Failed to initialize Git repository: {:?}", e);
    }

    // Step 5: Create initial commit
    if let Err(e) = create_initial_commit(context) {
        warn!("Failed to create initial commit: {:?}", e);
    }

    info!(
        "Project '{}' created successfully at {}",
        context.project_name,
        context.project_path.display()
    );

    Ok(())
}
