// src/create_project.rs
use crate::context::Context;
use crate::create_files::{create_cmakelists, create_source_files};
use crate::initialize_git::{create_initial_commit, initialize_git_repo, JuMakeError};
use std::fs;
use log::{info, warn, error};

pub fn create_project(context: &Context) -> Result<(), JuMakeError> {
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

    // Create project directory
    fs::create_dir_all(&context.project_path)
        .map_err(JuMakeError::Io)?;

    // Create CMakeLists.txt
    if let Err(e) = create_cmakelists(context) {
        warn!("Failed to create CMakeLists.txt: {}", e);
    }

    // Create source files
    if let Err(e) = create_source_files(context) {
        warn!("Failed to create source files: {}", e);
    }

    // Initialize Git repository
    if let Err(e) = initialize_git_repo(context) {
        warn!("Failed to initialize Git repository: {:?}", e);
    }

    // Create initial commit
    if let Err(e) = create_initial_commit(context) {
        warn!("Failed to create initial commit: {:?}", e);
    }

    info!("Project '{}' created successfully at {}", context.project_name, context.project_path.display());
    Ok(())
}
