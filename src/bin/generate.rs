//! TypeSpec Code Generator CLI

use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use typespec_api::{
    codegen::{Generator, Language, Side},
    parse, TypeSpecFile,
};

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

fn main() -> Result<()> {
    let cli = Cli::parse();

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

    println!("Generated {} files:", generated.len());
    for path in &generated {
        println!("  {}", path);
    }

    Ok(())
}
