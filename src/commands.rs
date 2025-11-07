//! Command parsing and dispatch logic.
/*
use crate::{
    context::{Context, current_context, current_context_with_build},
    utils::{resolve_project_path, select_template, validate_build_type, save_build_type, get_effective_build_type, determine_template_name, extract_project_name},
    create_project::create_project,
    create_files::add_class,
    build::{build_project, run_project},
};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use anyhow::{Result, Context as AnyhowContext};
use log::info;

/// Main CLI parser
#[derive(Parser)]
#[command(author, version, about = "A CLI tool for creating and managing JUCE projects.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// CLI subcommands
#[derive(Subcommand)]
pub enum Commands {
    /// Create a new JUCE project
    New {
        project_name: String,
        #[arg(short, long)]
        path: Option<String>,
        #[arg(short, long)]
        template: Option<String>,
    },

    /// Add a new element (class/component)
    Add {
        #[arg(value_enum)]
        element_type: ElementType,
        element_name: String,
    },

    /// Build the current project
    Build {
        #[arg(short = 't', long = "build-type", default_value_t = String::from("Release"))]
        build_type: String,
    },

    /// Run the current project
    Run {
        #[arg(short = 't', long = "build-type", default_value = "LastUsed")]
        build_type: String,
    },
}

/// Type of element to add
#[derive(ValueEnum, Clone, Debug)]
pub enum ElementType {
    Class,
    Component,
}

/// Dispatch the CLI commands to their respective handlers
pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::New { project_name, path, template } => handle_new(project_name, path, template),
        Commands::Add { element_type, element_name } => handle_add(element_type, element_name),
        Commands::Build { build_type } => handle_build(build_type),
        Commands::Run { build_type } => handle_run(build_type),
    }
}

// ------------------------
// Command Handlers
// ------------------------

fn handle_new(project_name: String, path: Option<String>, template: Option<String>) -> Result<()> {
    let project_path = resolve_project_path(&project_name, path)?;
    let template_name = template.or_else(select_template);

    let context = Context {
        project_name,
        project_path,
        template_name,
        build_type: "Release".into(),
    };

    create_project(&context)
        .with_context(|| format!("Failed to create project at {}", context.project_path.display()))?;

    info!("✅ Project created at {}", context.project_path.display());
    Ok(())
}

fn handle_add(element_type: ElementType, element_name: String) -> Result<()> {
    let context = current_context()?;
    add_class(&context, &format!("{element_type:?}"), &element_name)
        .with_context(|| format!("Failed to add {} '{}'", format!("{element_type:?}"), element_name))?;
    info!("✅ Added {} '{}'", format!("{element_type:?}"), element_name);
    Ok(())
}

fn handle_build(build_type: String) -> Result<()> {
    validate_build_type(&build_type)?;

    let context = current_context_with_build(&build_type)?;
    build_project(&context)?;
    save_build_type(&context)?;
    info!("✅ Build succeeded: {}", build_type);
    Ok(())
}

fn handle_run(build_type: String) -> Result<()> {
    let project_path = std::env::current_dir()?;
    let effective_build_type = get_effective_build_type(&build_type, &project_path);

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
