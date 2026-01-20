//! TypeSpec Code Generator CLI

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;
use typespec_api::{
    codegen::{Generator, Language, Side},
    parse, TypeSpecFile,
};

/// Global flag for watch mode termination
static RUNNING: AtomicBool = AtomicBool::new(true);

#[derive(Parser)]
#[command(name = "tsp-gen")]
#[command(about = "Generate code from TypeSpec definitions")]
struct Cli {
    /// Input TypeSpec file(s)
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Output directory
    #[arg(short, long, default_value = "generated")]
    output: PathBuf,

    /// Target language
    #[arg(short, long, value_enum)]
    language: Language,

    /// Generate client, server, or both
    #[arg(short, long, value_enum, default_value = "both")]
    side: Side,

    /// Package name for generated code
    #[arg(short, long, default_value = "api")]
    package: String,

    /// Watch input files and regenerate on changes
    #[arg(short, long)]
    watch: bool,
}

/// Recursively resolve imports from a TypeSpec file
fn resolve_imports(
    file: TypeSpecFile,
    base_path: &Path,
    resolved: &mut HashSet<PathBuf>,
) -> Result<TypeSpecFile> {
    let mut combined = TypeSpecFile {
        imports: Vec::new(), // Don't carry forward imports
        usings: file.usings,
        namespace: file.namespace,
        declarations: file.declarations,
    };

    // Process each import
    for import in file.imports {
        // Skip TypeSpec standard library imports
        if import.path.starts_with("@typespec/") {
            continue;
        }

        // Resolve the import path relative to the current file
        let import_path = if import.path.starts_with("./") || import.path.starts_with("../") {
            base_path.join(&import.path)
        } else {
            base_path.join(&import.path)
        };

        // Normalize path and add .tsp extension if missing
        let import_path = if import_path.extension().is_none() {
            import_path.with_extension("tsp")
        } else {
            import_path
        };

        // Canonicalize to handle .. and .
        let import_path = import_path.canonicalize().unwrap_or(import_path);

        // Skip if already resolved (prevents circular imports)
        if resolved.contains(&import_path) {
            continue;
        }
        resolved.insert(import_path.clone());

        // Read and parse the imported file
        if import_path.exists() {
            let source = std::fs::read_to_string(&import_path)
                .with_context(|| format!("Failed to read import {}", import_path.display()))?;

            let imported = parse(&source)
                .with_context(|| format!("Failed to parse import {}", import_path.display()))?;

            // Recursively resolve imports from the imported file
            let import_dir = import_path.parent().unwrap_or(Path::new("."));
            let resolved_import = resolve_imports(imported, import_dir, resolved)?;

            // Merge declarations from the imported file
            combined.usings.extend(resolved_import.usings);
            combined.declarations.extend(resolved_import.declarations);

            // Don't override namespace from imports
        }
    }

    Ok(combined)
}

/// Perform a single generation run
fn do_generate(cli: &Cli) -> Result<Vec<String>> {
    // Parse all input files with import resolution
    let mut combined = TypeSpecFile::default();
    let mut resolved = HashSet::new();

    for input in &cli.input {
        let canonical = input
            .canonicalize()
            .with_context(|| format!("Failed to resolve path {}", input.display()))?;

        // Skip if already processed
        if resolved.contains(&canonical) {
            continue;
        }
        resolved.insert(canonical.clone());

        let source = std::fs::read_to_string(&canonical)
            .with_context(|| format!("Failed to read {}", input.display()))?;

        let file =
            parse(&source).with_context(|| format!("Failed to parse {}", input.display()))?;

        // Resolve imports relative to the input file's directory
        let base_dir = canonical.parent().unwrap_or(Path::new("."));
        let resolved_file = resolve_imports(file, base_dir, &mut resolved)?;

        // Merge declarations
        combined.usings.extend(resolved_file.usings);
        combined.declarations.extend(resolved_file.declarations);

        if resolved_file.namespace.is_some() {
            combined.namespace = resolved_file.namespace;
        }
    }

    // Generate code
    let output_dir = cli.output.join(match cli.language {
        Language::Python => "python",
        Language::TypeScript => "typescript",
        Language::Rust => "rust",
        Language::OpenApi => "openapi",
    });

    let generator = Generator::new(&combined, &output_dir, &cli.package);
    let generated = generator.generate(cli.language, cli.side)?;

    Ok(generated)
}

/// Run in watch mode
fn run_watch(cli: &Cli) -> Result<()> {
    // Reset running flag
    RUNNING.store(true, Ordering::SeqCst);

    // Set up Ctrl+C handler
    let _ = ctrlc::set_handler(|| {
        RUNNING.store(false, Ordering::SeqCst);
    });

    // Collect directories to watch (parent dirs of input files)
    let watch_dirs: HashSet<PathBuf> = cli
        .input
        .iter()
        .filter_map(|f| {
            f.canonicalize()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        })
        .collect();

    if watch_dirs.is_empty() {
        anyhow::bail!("No valid directories to watch");
    }

    // Initial generation
    println!("TypeSpec Generator - Watch Mode");
    println!("================================\n");

    print!("Running initial generation... ");
    let _ = io::stdout().flush();

    match do_generate(cli) {
        Ok(files) => {
            println!("done");
            println!("Generated {} files:", files.len());
            for path in &files {
                println!("  {}", path);
            }
            println!();
        }
        Err(e) => println!("failed\nError: {}\n", e),
    }

    println!(
        "Watching {} director{} for changes:",
        watch_dirs.len(),
        if watch_dirs.len() == 1 { "y" } else { "ies" }
    );
    for dir in &watch_dirs {
        println!("  {}", dir.display());
    }
    println!("\nPress Ctrl+C to stop\n");

    // Create watcher
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        Config::default().with_poll_interval(Duration::from_millis(500)),
    )?;

    // Watch all directories
    for dir in &watch_dirs {
        watcher.watch(dir, RecursiveMode::Recursive)?;
    }

    // Watch loop
    while RUNNING.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                // Filter for .tsp file changes
                let tsp_changed = event
                    .paths
                    .iter()
                    .any(|p| p.extension().map(|e| e == "tsp").unwrap_or(false));

                if tsp_changed {
                    let timestamp = Local::now().format("%H:%M:%S");
                    println!("[{}] Change detected, regenerating...", timestamp);

                    match do_generate(cli) {
                        Ok(files) => {
                            println!("Generated {} files:", files.len());
                            for path in &files {
                                println!("  {}", path);
                            }
                            println!();
                        }
                        Err(e) => println!("Error: {}\n", e),
                    }
                }
            }
            Ok(Err(e)) => {
                eprintln!("Watch error: {}", e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout, continue loop
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    println!("\nWatch stopped.");
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.watch {
        run_watch(&cli)
    } else {
        let generated = do_generate(&cli)?;
        println!("Generated {} files:", generated.len());
        for path in &generated {
            println!("  {}", path);
        }
        Ok(())
    }
}
