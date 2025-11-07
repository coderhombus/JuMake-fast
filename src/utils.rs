//! Utility functions for JuMake
/*
use crate::context::Context;
use dialoguer::{theme::ColorfulTheme, Select};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context as AnyhowContext};
use std::io::BufRead;

/// Resolve project path and prevent double folder creation
pub fn resolve_project_path(project_name: &str, path: Option<String>) -> Result<PathBuf> {
    let base_path = path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));
    Ok(base_path.join(project_name))
}

/// Prompt user to select template
pub fn select_template() -> Option<String> {
    let options = ["GuiApplication", "AudioPlugin", "ConsoleApp"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a template:")
        .default(0)
        .items(&options)
        .interact()
        .ok()?;
    Some(options[selection].to_string())
}

/// Validate build type
pub fn validate_build_type(build_type: &str) -> Result<()> {
    match build_type {
        "Debug" | "Release" | "RelWithDebInfo" | "MinSizeRel" | "LastUsed" => Ok(()),
        _ => anyhow::bail!(
            "Invalid build type '{}'. Options: Debug, Release, RelWithDebInfo, MinSizeRel, LastUsed",
            build_type
        ),
    }
}

/// Get effective build type (considering LastUsed)
pub fn get_effective_build_type(build_type: &str, project_path: &Path) -> String {
    if build_type == "LastUsed" {
        fs::read_to_string(project_path.join(".jumake")).unwrap_or_else(|_| "Release".into())
    } else {
        build_type.to_string()
    }
}

/// Determine template from CMakeLists.txt
pub fn determine_template_name(project_path: &Path) -> Option<String> {
    let cmakelists_path = project_path.join("src").join("CMakeLists.txt");
    if cmakelists_path.exists() {
        let content = fs::read_to_string(&cmakelists_path).unwrap_or_default();
        let re = Regex::new(r#"set\(JUMAKE_TEMPLATE\s+"([^"]+)"\)"#).unwrap();
        if let Some(caps) = re.captures(&content) {
            return Some(caps[1].to_string());
        }
    }
    Some("GuiApplication".into())
}

/// Extract project name from CMakeLists.txt
pub fn extract_project_name<P: AsRef<Path>>(cmake_file_path: P) -> Result<String> {
    let file = fs::File::open(&cmake_file_path)?;
    for line in std::io::BufReader::new(file).lines() {
        let line = line?;
        if let Some(stripped) = line.trim_start().strip_prefix("project(") {
            if let Some(end) = stripped.find(')') {
                let content = &stripped[..end];
                if let Some(name) = content.split_whitespace().next() {
                    return Ok(name.to_string());
                }
            }
        }
    }
    anyhow::bail!("Project name not found in {:?}", cmake_file_path.as_ref());
}

/// Get current context
pub fn current_context() -> Result<Context> {
    let project_path = std::env::current_dir()?;
    Ok(Context {
        project_name: project_path.file_name().unwrap().to_string_lossy().into(),
        project_path,
        template_name: None,
        build_type: "Release".into(),
    })
}

/// Get current context with a specified build
pub fn current_context_with_build(build_type: &str) -> Result<Context> {
    let project_path = std::env::current_dir()?;
    Ok(Context {
        project_name: project_path.file_name().unwrap().to_string_lossy().into(),
        project_path: project_path.clone(),
        template_name: determine_template_name(&project_path),
        build_type: build_type.into(),
    })
}

/// Save build type to .jumake
pub fn save_build_type(context: &Context) -> Result<()> {
    fs::write(context.project_path.join(".jumake"), &context.build_type)?;
    Ok(())
}
