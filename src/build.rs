// src/build.rs

use crate::context::Context;
use std::fs;
use std::process::{Command, Stdio};
use std::str;
// use std::path::PathBuf;
use thiserror::Error; // For structured errors
use which::which;

/// Custom error type for structured error handling
#[derive(Error, Debug)]
pub enum BuildError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("CMake configuration failed")]
    CMakeConfigureFailed,
    #[error("CMake build failed")]
    CMakeBuildFailed,
    #[error("Executable not found for build type: {0}")]
    ExecutableNotFound(String),
    #[error("compile_commands.json not found")]
    CompileCommandsMissing,
}

/// Build the project using CMake, optionally leveraging ccache.
pub fn build_project(context: &Context) -> Result<(), BuildError> {
    println!("Building project '{}' in '{}'...", context.project_name, context.build_type);

    let build_dir = context.project_path.join("jumake_build");
    fs::create_dir_all(&build_dir)?; // Ensure build directory exists

    // Prefer Ninja if installed, fallback to Unix Makefiles
    let generator = if Command::new("ninja").output().is_ok() {
        "Ninja"
    } else {
        "Unix Makefiles"
    };

    let cmake_cache = build_dir.join("CMakeCache.txt");

    // Only configure CMake if cache doesn't exist
    if !cmake_cache.exists() {
        println!("Running CMake configuration...");

        let ccache_enabled = which("ccache").is_ok();
        if ccache_enabled {
            println!("⚡ Detected ccache — enabling compiler caching!");
        } else {
            println!("⚠️  ccache not found — building without compiler cache.");
        }

        let mut cmake_cmd = Command::new("cmake");
        cmake_cmd
            .arg("..")
            .arg(format!("-G{}", generator))
            .arg(format!("-DCMAKE_BUILD_TYPE={}", context.build_type))
            .arg("-DCMAKE_EXPORT_COMPILE_COMMANDS=ON");

        if ccache_enabled {
            cmake_cmd
                .arg("-DCMAKE_C_COMPILER_LAUNCHER=ccache")
                .arg("-DCMAKE_CXX_COMPILER_LAUNCHER=ccache");
        }

        let status = cmake_cmd
            .current_dir(&build_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(BuildError::CMakeConfigureFailed);
        }
    } else {
        println!("CMake already configured, skipping configure step...");
    }

    // Build the project
    
    let num_cpus = std::cmp::max(num_cpus::get() - 2, 2);
    let status = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .arg("--config")
        .arg(&context.build_type)
        .arg("--parallel")
        .arg(num_cpus.to_string())
        .current_dir(&build_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(BuildError::CMakeBuildFailed);
    }

    // Move compile_commands.json to project root (non-Windows)
    if !cfg!(target_os = "windows") {
        let compile_commands_path = build_dir.join("compile_commands.json");
        if compile_commands_path.exists() {
            fs::copy(&compile_commands_path, &context.project_path.join("compile_commands.json"))?;
            println!("Moved compile_commands.json to the project root.");
        } else {
            return Err(BuildError::CompileCommandsMissing);
        }
    }

    println!("Build successful!");
    Ok(())
}

/// Run the built project executable
pub fn run_project(context: &Context) -> Result<(), BuildError> {
    // Ensure project is built first
    build_project(context)?;

    println!("Running project '{}'...", context.project_name);

    let executable_path = find_executable(context)?;

    // MacOS special handling for non-console apps
    if cfg!(target_os = "macos") && context.template_name.as_deref() != Some("ConsoleApp") {
        Command::new("open")
            .arg(executable_path)
            .status()?;
    } else {
        Command::new(executable_path)
            .current_dir(context.project_path.join("jumake_build"))
            .status()?;
    }

    println!("Execution completed.");
    Ok(())
}

/// Find the project executable in the build directory
fn find_executable(context: &Context) -> Result<String, BuildError> {
    let build_dir = context.project_path.join("jumake_build");

    // Prepare OS-specific find commands
    let output = if cfg!(target_os = "windows") {
        let cmd = format!(
            "Get-ChildItem -Recurse -Filter '{}.exe' -File | Select-Object -ExpandProperty FullName",
            context.project_name
        );
        Command::new("powershell")
            .arg("-Command")
            .arg(&cmd)
            .current_dir(&build_dir)
            .output()?
    } else {
        let cmd = match (cfg!(target_os = "macos"), context.template_name.as_deref()) {
            (true, Some("AudioPlugin")) => {
                format!("find {} -name {} -type f -perm +111 | grep Standalone", build_dir.to_string_lossy(), context.project_name)
            }
            (true, _) => format!("find {} -name {} -type f -perm +111", build_dir.to_string_lossy(), context.project_name),
            _ => format!("find {} -name {} -type f -executable", build_dir.to_string_lossy(), context.project_name),
        };
        Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .output()?
    };

    if !output.status.success() {
        return Err(BuildError::ExecutableNotFound(context.build_type.clone()));
    }

    let paths: Vec<&str> = str::from_utf8(&output.stdout)?.lines().collect();

    let executable_path = paths
        .into_iter()
        .find(|path| path.contains(&context.build_type))
        .ok_or_else(|| BuildError::ExecutableNotFound(context.build_type.clone()))?;

    // On macOS, truncate path after ".app"
    let executable_path = if cfg!(target_os = "macos") {
        executable_path.split(".app").next().map(|s| format!("{}.app", s)).unwrap_or_else(|| executable_path.to_string())
    } else {
        executable_path.to_string()
    };

    println!("Executable path: {}", executable_path);
    Ok(executable_path)
}
