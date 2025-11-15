// src/create_files.rs
//! This module provides functions to create source files and CMakeLists for projects
//! and to add new classes or components based on templates.

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path};
use indoc::indoc;
use crate::context::Context;
use anyhow::{Context as AnyhowContext, Result};

/// Creates source files in the project based on the template specified in the context.
pub fn create_source_files(context: &Context) -> Result<()> {
    let src_path = context.project_path.join("src");

    // Ensure the `src` directory exists
    fs::create_dir_all(&src_path)
        .with_context(|| format!("Failed to create directory: {}", src_path.display()))?;

    match context.template_name.as_deref() {
        Some("GuiApplication") => {
            create_file_from_template(&src_path, "Main.cpp", MAIN_CPP_TEMPLATE)?;
            create_file_from_template(&src_path, "MainComponent.cpp", MAIN_COMPONENT_CPP_TEMPLATE)?;
            create_file_from_template(&src_path, "MainComponent.h", MAIN_COMPONENT_H_TEMPLATE)?;
            create_file_from_template(&src_path, "CMakeLists.txt", GUI_APP_CMAKE_TEMPLATE)?;
        }
        Some("AudioPlugin") => {
            create_file_from_template(&src_path, "PluginProcessor.cpp", PLUGIN_PROCESSOR_CPP_TEMPLATE)?;
            create_file_from_template(&src_path, "PluginProcessor.h", PLUGIN_PROCESSOR_H_TEMPLATE)?;
            create_file_from_template(&src_path, "PluginEditor.cpp", PLUGIN_EDITOR_CPP_TEMPLATE)?;
            create_file_from_template(&src_path, "PluginEditor.h", PLUGIN_EDITOR_H_TEMPLATE)?;
            create_file_from_template(&src_path, "CMakeLists.txt", AUDIO_PLUGIN_CMAKE_TEMPLATE)?;
        }
        Some("ConsoleApp") => {
            create_file_from_template(&src_path, "Main.cpp", CONSOLE_APP_MAIN_CPP_TEMPLATE)?;
            create_file_from_template(&src_path, "CMakeLists.txt", CONSOLE_APP_CMAKE_TEMPLATE)?;
        }
        Some(template) => anyhow::bail!("Unknown template: {}", template),
        None => anyhow::bail!("No template specified in the context"),
    }

    Ok(())
}

/// Adds a new class or component to the project.
pub fn add_class(context: &Context, element_type: &str, element_name: &str) -> Result<()> {
    let src_path = context.project_path.join("src");

    // Determine templates and adjusted name
    let (header_template, cpp_template, adjusted_name) = match element_type {
        "class" => (CLASS_H_TEMPLATE, CLASS_CPP_TEMPLATE, element_name.to_string()),
        "component" => {
            let mut name = element_name.to_string();
            name.push_str("Component");
            (COMPONENT_H_TEMPLATE, COMPONENT_CPP_TEMPLATE, name)
        }
        _ => anyhow::bail!("Invalid element type: {}", element_type),
    };

    let header_file_name = format!("{}.h", adjusted_name);
    let cpp_file_name = format!("{}.cpp", adjusted_name);
    let header_path = src_path.join(&header_file_name);
    let cpp_path = src_path.join(&cpp_file_name);

    // Prevent overwriting existing files
    if header_path.exists() || cpp_path.exists() {
        anyhow::bail!("{} '{}' already exists in the project.", element_type, adjusted_name);
    }

    // Create files from templates
    create_classfile_from_template(&src_path, &header_file_name, header_template, &adjusted_name)?;
    create_classfile_from_template(&src_path, &cpp_file_name, cpp_template, &adjusted_name)?;

    // Update CMakeLists.txt
    update_cmakelists(&src_path, &cpp_file_name)?;

    println!("{} '{}' added successfully!", element_type, adjusted_name);
    Ok(())
}

/// Creates a file from a template, replacing "Template" with `element_name`.
fn create_classfile_from_template(
    src_path: &Path,
    file_name: &str,
    template: &[u8],
    element_name: &str,
) -> Result<()> {
    let path = src_path.join(file_name);
    let content = String::from_utf8_lossy(template).replace("Template", element_name);
    fs::write(&path, content.as_bytes())
        .with_context(|| format!("Failed to create file: {}", path.display()))?;
    println!("Created file: {}", path.display());
    Ok(())
}

/// Creates a file from a template without modifications.
fn create_file_from_template(src_path: &Path, file_name: &str, template: &[u8]) -> Result<()> {
    let path = src_path.join(file_name);
    fs::write(&path, template)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;
    println!("Created file: {}", path.display());
    Ok(())
}

/// Updates `CMakeLists.txt` to include the newly created cpp file under `PRIVATE`.
fn update_cmakelists(src_path: &Path, cpp_file_name: &str) -> Result<()> {
    let cmakelists_path = src_path.join("CMakeLists.txt");
    let file = File::open(&cmakelists_path)
        .with_context(|| format!("Failed to open CMakeLists.txt at {}", cmakelists_path.display()))?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    let mut new_lines = Vec::with_capacity(lines.len() + 1);
    let mut found_target_sources = false;
    let mut added = false;

    for line in &lines {
        new_lines.push(line.clone());

        if line.trim_start().starts_with("target_sources(${PROJECT_NAME}") {
            found_target_sources = true;
        }

        if found_target_sources && line.trim_start().starts_with("PRIVATE") && !added {
            // Calculate indentation and insert the cpp file
            let indentation = line.chars().take_while(|c| c.is_whitespace()).count() + 4;
            let new_cpp_line = format!("{:indent$}{}", "", cpp_file_name, indent = indentation);
            new_lines.push(new_cpp_line);
            added = true;
        }
    }

    if !added {
        anyhow::bail!("Could not find 'PRIVATE' after 'target_sources' in CMakeLists.txt");
    }

    fs::write(&cmakelists_path, new_lines.join("\n"))
        .with_context(|| format!("Failed to update CMakeLists.txt at {}", cmakelists_path.display()))?;
    Ok(())
}

/// Creates a basic `CMakeLists.txt` for the project.
pub fn create_cmakelists(context: &Context) -> Result<()> {
    let cmakelists_path = context.project_path.join("CMakeLists.txt");

    let cmake_content = format!(
        indoc! {"
            cmake_minimum_required(VERSION 3.24)
            project({} VERSION 0.0.1)
            add_subdirectory(modules/JUCE)
            add_subdirectory(src)
        "},
        context.project_name
    );

    fs::write(&cmakelists_path, cmake_content.as_bytes())
        .with_context(|| format!("Failed to create CMakeLists.txt at {}", cmakelists_path.display()))?;
    Ok(())
}

// ======================= TEMPLATES ========================
const MAIN_CPP_TEMPLATE: &[u8] = include_bytes!("../templates/GuiApplicationTemplate/Main.cpp.template");
const MAIN_COMPONENT_CPP_TEMPLATE: &[u8] = include_bytes!("../templates/GuiApplicationTemplate/MainComponent.cpp.template");
const MAIN_COMPONENT_H_TEMPLATE: &[u8] = include_bytes!("../templates/GuiApplicationTemplate/MainComponent.h.template");
const GUI_APP_CMAKE_TEMPLATE: &[u8] = include_bytes!("../templates/GuiApplicationTemplate/CMakeLists.txt.template");

const PLUGIN_PROCESSOR_CPP_TEMPLATE: &[u8] = include_bytes!("../templates/AudioPluginTemplate/PluginProcessor.cpp.template");
const PLUGIN_PROCESSOR_H_TEMPLATE: &[u8] = include_bytes!("../templates/AudioPluginTemplate/PluginProcessor.h.template");
const PLUGIN_EDITOR_CPP_TEMPLATE: &[u8] = include_bytes!("../templates/AudioPluginTemplate/PluginEditor.cpp.template");
const PLUGIN_EDITOR_H_TEMPLATE: &[u8] = include_bytes!("../templates/AudioPluginTemplate/PluginEditor.h.template");
const AUDIO_PLUGIN_CMAKE_TEMPLATE: &[u8] = include_bytes!("../templates/AudioPluginTemplate/CMakeLists.txt.template");

const CONSOLE_APP_MAIN_CPP_TEMPLATE: &[u8] = include_bytes!("../templates/ConsoleAppTemplate/Main.cpp.template");
const CONSOLE_APP_CMAKE_TEMPLATE: &[u8] = include_bytes!("../templates/ConsoleAppTemplate/CMakeLists.txt.template");

const CLASS_H_TEMPLATE: &[u8] = include_bytes!("../templates/ClassTemplates/Class.h.template");
const CLASS_CPP_TEMPLATE: &[u8] = include_bytes!("../templates/ClassTemplates/Class.cpp.template");
const COMPONENT_H_TEMPLATE: &[u8] = include_bytes!("../templates/ClassTemplates/Component.h.template");
const COMPONENT_CPP_TEMPLATE: &[u8] = include_bytes!("../templates/ClassTemplates/Component.cpp.template");
