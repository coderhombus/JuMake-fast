// src/main.rs

//! CLI entrypoint for JuMake: create, build, run, and manage JUCE projects.

use clap::{Parser, Subcommand, ValueEnum};
use dialoguer::{theme::ColorfulTheme, Select};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::error::Error;
use std::io::{BufRead, BufReader};
use log::info;

mod build;
mod context;
mod create_project;
mod create_files;
mod initialize_git;

use build::{build_project, run_project};
use context::Context;
use create_project::create_project;
use create_files::add_class;

/// Main CLI parser
#[derive(Parser)]
#[command(
    author,
    version,
    about = "A CLI tool for creating and managing JUCE projects."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// CLI subcommands
#[derive(Subcommand)]
enum Commands {
    /// Create a new JUCE project
    New {
        project_name: String,
        #[arg(short, long)]
        path: Option<String>,
        #[arg(short, long)]
        template: Option<String>,
    },
    /// Add a new C++ class or JUCE component
    Add {
        #[arg(value_enum)]
        element_type: ElementType,
        element_name: String,
    },
    /// Build the project
    Build {
        #[arg(short = 't', long = "build-type", default_value_t = String::from("Release"))]
        build_type: String,
    },
    /// Build and run the project
    Run {
        #[arg(short = 't', long = "build-type", default_value = "LastUsed")]
        build_type: String,
    },
}

/// Strongly-typed element type for `Add` command
#[derive(ValueEnum, Clone, Debug)]
enum ElementType {
    Class,
    Component,
}

fn main() {
    env_logger::init(); // Initialize logger
    let cli = Cli::parse();

    // Execute selected command and handle errors gracefully
    if let Err(e) = match cli.command {
        Commands::New { project_name, path, template } => handle_new(project_name, path, template),
        Commands::Add { element_type, element_name } => handle_add(element_type, element_name),
        Commands::Build { build_type } => handle_build(build_type),
        Commands::Run { build_type } => handle_run(build_type),
    } {
        eprintln!("❌ Error: {}", e);
    }
}

// ------------------------
// Command handlers
// ------------------------

fn handle_new(project_name: String, path: Option<String>, template: Option<String>) -> Result<(), Box<dyn Error>> {
    // Determine project path
    let project_path = path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get cwd"))
        .join(&project_name);

    // Use provided template or prompt user
    let template_name = template.or_else(|| select_template());

    let context = Context {
        project_name,
        project_path,
        template_name,
        build_type: "Release".to_string(),
    };

    create_project(&context)?;
    info!("✅ Project created successfully at {}", context.project_path.display());
    Ok(())
}

fn handle_add(element_type: ElementType, element_name: String) -> Result<(), Box<dyn Error>> {
    let context = current_context()?;
    // map the enum to the lowercase strings expected by add_class
    let element_type_str = match element_type {
        ElementType::Class => "class",
        ElementType::Component => "component",
    };
    add_class(&context, element_type_str, &element_name)?;
    info!("✅ Added {}: {}", element_type_str, element_name);
    Ok(())
}

fn handle_build(build_type: String) -> Result<(), Box<dyn Error>> {
    validate_build_type(&build_type)?;

    let context = current_context_with_build(&build_type)?;
    build_project(&context)?;
    save_build_type(&context)?;
    info!("✅ Build succeeded: {}", build_type);
    Ok(())
}

fn handle_run(build_type: String) -> Result<(), Box<dyn Error>> {
    let project_path = std::env::current_dir()?;
// Use last build type if requested
    let effective_build_type = if build_type == "LastUsed" {
        read_last_build_type(&project_path).unwrap_or_else(|| "Release".to_string())
    } else {
        build_type
    };

    validate_build_type(&effective_build_type)?;

    let project_name = extract_project_name(project_path.join("CMakeLists.txt"))?;
    let context = Context {
        project_name,
        project_path: project_path.clone(),
        template_name: determine_template_name(&project_path),
        build_type: effective_build_type,
    };

    run_project(&context)?;
    info!("✅ Run completed.");
    Ok(())
}

// ------------------------
// Helpers
// ------------------------

/// Get current context using working directory
fn current_context() -> Result<Context, Box<dyn Error>> {
    let project_path = std::env::current_dir()?;
    Ok(Context {
        project_name: project_path.file_name().unwrap().to_string_lossy().to_string(),
        project_path,
        template_name: None,
        build_type: "Release".to_string(),
    })
}

/// Get current context with specified build type
fn current_context_with_build(build_type: &str) -> Result<Context, Box<dyn Error>> {
    let project_path = std::env::current_dir()?;
    Ok(Context {
        project_name: project_path.file_name().unwrap().to_string_lossy().to_string(),
        project_path: project_path.clone(),
        template_name: determine_template_name(&project_path),
        build_type: build_type.to_string(),
    })
}

/// Prompt user to select a template interactively
fn select_template() -> Option<String> {
    let options = ["GuiApplication", "AudioPlugin", "ConsoleApp"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a template:")
        .default(0)
        .items(&options)
        .interact()
        .ok()?;
    Some(options[selection].to_string())
}

/// Validate build type string
fn validate_build_type(build_type: &str) -> Result<(), String> {
    match build_type {
        "Debug" | "Release" | "RelWithDebInfo" | "MinSizeRel" => Ok(()),
        _ => Err(format!(
            "Invalid build type: {}. Valid options: Debug, Release, RelWithDebInfo, MinSizeRel",
            build_type
        )),
    }
}

/// Persist last used build type
fn save_build_type(context: &Context) -> std::io::Result<()> {
    fs::write(context.project_path.join(".jumake"), &context.build_type)
}

/// Read last used build type
fn read_last_build_type(project_path: &Path) -> Option<String> {
    fs::read_to_string(project_path.join(".jumake")).ok()
}

/// Determine template name from CMakeLists.txt
fn determine_template_name(project_path: &Path) -> Option<String> {
    let cmakelists_path = project_path.join("src").join("CMakeLists.txt");
    if cmakelists_path.exists() {
        let content = fs::read_to_string(&cmakelists_path).unwrap_or_default();
        let re = Regex::new(r#"set\(JUMAKE_TEMPLATE\s+"([^"]+)"\)"#).unwrap();
        if let Some(caps) = re.captures(&content) {
            return Some(caps[1].to_string());
        }
    }
    Some("GuiApplication".to_string())
}

/// Extract project name from CMakeLists.txt
fn extract_project_name<P: AsRef<Path>>(cmake_file_path: P) -> Result<String, Box<dyn Error>> {
    let file = fs::File::open(cmake_file_path)?;
    for line in BufReader::new(file).lines() {
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
    Err("Project name not found in CMakeLists.txt".into())
}
