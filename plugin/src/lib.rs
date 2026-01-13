//! TypeSpec Code Generator Plugin
//!
//! ADI plugin for generating code from TypeSpec definitions.

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError, ServiceHandle,
    ServiceMethod, ServiceVTable, ServiceVersion,
};
use serde_json::json;
use std::collections::HashSet;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use typespec_api::{
    codegen::{Generator, Language, Side},
    parse, TypeSpecFile,
};

/// Plugin-specific CLI service ID
const SERVICE_CLI: &str = "adi.tsp-gen.cli";

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "adi.tsp-gen",
        "TypeSpec Generator",
        env!("CARGO_PKG_VERSION"),
        "tools",
    )
    .with_author("ADI Team")
    .with_description("Generate code from TypeSpec definitions")
    .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        // Register CLI commands service
        let cli_descriptor =
            ServiceDescriptor::new(SERVICE_CLI, ServiceVersion::new(1, 0, 0), "adi.tsp-gen")
                .with_description("CLI commands for TypeSpec code generation");

        let cli_handle = ServiceHandle::new(
            SERVICE_CLI,
            ctx as *const c_void,
            &CLI_SERVICE_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(cli_descriptor, cli_handle) {
            host.error(&format!(
                "Failed to register CLI commands service: {}",
                code
            ));
            return code;
        }

        host.info("TypeSpec Generator plugin initialized");
    }

    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

// === Plugin Entry Point ===

static PLUGIN_VTABLE: PluginVTable = PluginVTable {
    info: plugin_info,
    init: plugin_init,
    update: ROption::RNone,
    cleanup: plugin_cleanup,
    handle_message: ROption::RNone,
};

#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &PLUGIN_VTABLE
}

// === CLI Service VTable ===

static CLI_SERVICE_VTABLE: ServiceVTable = ServiceVTable {
    invoke: cli_invoke,
    list_methods: cli_list_methods,
};

extern "C" fn cli_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    match method.as_str() {
        "run_command" => {
            let result = run_cli_command(args.as_str());
            match result {
                Ok(output) => RResult::ROk(RString::from(output)),
                Err(e) => RResult::RErr(ServiceError::invocation_error(e)),
            }
        }
        "list_commands" => {
            let commands = json!([
                {"name": "generate", "description": "Generate code from TypeSpec files", "usage": "generate <input...> -l <language> [-o <output>] [-s <side>] [-p <package>]"},
                {"name": "languages", "description": "List supported languages", "usage": "languages"},
                {"name": "help", "description": "Show help information", "usage": "help"}
            ]);
            RResult::ROk(RString::from(
                serde_json::to_string(&commands).unwrap_or_default(),
            ))
        }
        _ => RResult::RErr(ServiceError::method_not_found(method.as_str())),
    }
}

extern "C" fn cli_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("run_command").with_description("Run a CLI command"),
        ServiceMethod::new("list_commands").with_description("List available commands"),
    ]
    .into_iter()
    .collect()
}

fn run_cli_command(context_json: &str) -> Result<String, String> {
    let context: serde_json::Value =
        serde_json::from_str(context_json).map_err(|e| format!("Invalid context: {}", e))?;

    // Parse command and args from context
    let args: Vec<String> = context
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("");
    let cmd_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();

    match subcommand {
        "generate" | "gen" => cmd_generate(&cmd_args),
        "languages" | "langs" => cmd_languages(),
        "help" | "" => cmd_help(),
        _ => Err(format!("Unknown command: {}", subcommand)),
    }
}

// === Command Implementations ===

fn cmd_generate(args: &[&str]) -> Result<String, String> {
    // Parse arguments
    let mut input_files: Vec<PathBuf> = Vec::new();
    let mut output_dir = PathBuf::from("generated");
    let mut language: Option<Language> = None;
    let mut side = Side::Both;
    let mut package = String::from("api");

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "-l" | "--language" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --language".to_string());
                }
                language = Some(parse_language(args[i + 1])?);
                i += 2;
            }
            "-o" | "--output" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --output".to_string());
                }
                output_dir = PathBuf::from(args[i + 1]);
                i += 2;
            }
            "-s" | "--side" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --side".to_string());
                }
                side = parse_side(args[i + 1])?;
                i += 2;
            }
            "-p" | "--package" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --package".to_string());
                }
                package = args[i + 1].to_string();
                i += 2;
            }
            arg if arg.starts_with('-') => {
                return Err(format!("Unknown option: {}", arg));
            }
            _ => {
                input_files.push(PathBuf::from(args[i]));
                i += 1;
            }
        }
    }

    if input_files.is_empty() {
        return Err(
            "No input files specified. Usage: generate <input...> -l <language>".to_string(),
        );
    }

    let language = language.ok_or("Missing required option: --language (-l)")?;

    // Parse all input files with import resolution
    let mut combined = TypeSpecFile::default();
    let mut resolved = HashSet::new();

    for input in &input_files {
        let canonical = input
            .canonicalize()
            .map_err(|e| format!("Failed to resolve path {}: {}", input.display(), e))?;

        // Skip if already processed
        if resolved.contains(&canonical) {
            continue;
        }
        resolved.insert(canonical.clone());

        let source = std::fs::read_to_string(&canonical)
            .map_err(|e| format!("Failed to read {}: {}", input.display(), e))?;

        let file =
            parse(&source).map_err(|e| format!("Failed to parse {}: {}", input.display(), e))?;

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
    let output_subdir = output_dir.join(match language {
        Language::Python => "python",
        Language::TypeScript => "typescript",
        Language::Rust => "rust",
        Language::OpenApi => "openapi",
    });

    let generator = Generator::new(&combined, &output_subdir, &package);
    let generated = generator
        .generate(language, side)
        .map_err(|e| format!("Code generation failed: {}", e))?;

    let mut output = format!("Generated {} files:\n", generated.len());
    for path in &generated {
        output.push_str(&format!("  {}\n", path));
    }

    Ok(output.trim_end().to_string())
}

fn cmd_languages() -> Result<String, String> {
    let output = r#"Supported languages:
  python     - Python client/server code
  typescript - TypeScript client/server code
  rust       - Rust client/server code
  openapi    - OpenAPI 3.0 specification (JSON + YAML)

Aliases:
  py  -> python
  ts  -> typescript
  rs  -> rust
  oas -> openapi"#;
    Ok(output.to_string())
}

fn cmd_help() -> Result<String, String> {
    let help = r#"TypeSpec Generator - Generate code from TypeSpec definitions

Usage: adi run adi.tsp-gen <command> [options]

Commands:
  generate   Generate code from TypeSpec files
  languages  List supported target languages
  help       Show this help message

Generate Options:
  <input...>            Input TypeSpec file(s)
  -l, --language <lang> Target language (required)
  -o, --output <dir>    Output directory (default: generated)
  -s, --side <side>     Generate client, server, or both (default: both)
  -p, --package <name>  Package name for generated code (default: api)

Examples:
  adi run adi.tsp-gen generate api.tsp -l python
  adi run adi.tsp-gen generate *.tsp -l typescript -o src/generated -s client
  adi run adi.tsp-gen generate main.tsp -l rust -p my_api
  adi run adi.tsp-gen generate spec.tsp -l openapi -p "My API v1""#;
    Ok(help.to_string())
}

// === Helper Functions ===

fn parse_language(s: &str) -> Result<Language, String> {
    match s.to_lowercase().as_str() {
        "python" | "py" => Ok(Language::Python),
        "typescript" | "ts" => Ok(Language::TypeScript),
        "rust" | "rs" => Ok(Language::Rust),
        "openapi" | "oas" => Ok(Language::OpenApi),
        _ => Err(format!(
            "Unknown language: {}. Use: python, typescript, rust, or openapi",
            s
        )),
    }
}

fn parse_side(s: &str) -> Result<Side, String> {
    match s.to_lowercase().as_str() {
        "client" => Ok(Side::Client),
        "server" => Ok(Side::Server),
        "both" => Ok(Side::Both),
        _ => Err(format!("Unknown side: {}. Use: client, server, or both", s)),
    }
}

/// Recursively resolve imports from a TypeSpec file
fn resolve_imports(
    file: TypeSpecFile,
    base_path: &Path,
    resolved: &mut HashSet<PathBuf>,
) -> Result<TypeSpecFile, String> {
    let mut combined = TypeSpecFile {
        imports: Vec::new(),
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
        let import_path = base_path.join(&import.path);

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
                .map_err(|e| format!("Failed to read import {}: {}", import_path.display(), e))?;

            let imported = parse(&source)
                .map_err(|e| format!("Failed to parse import {}: {}", import_path.display(), e))?;

            // Recursively resolve imports from the imported file
            let import_dir = import_path.parent().unwrap_or(Path::new("."));
            let resolved_import = resolve_imports(imported, import_dir, resolved)?;

            // Merge declarations from the imported file
            combined.usings.extend(resolved_import.usings);
            combined.declarations.extend(resolved_import.declarations);
        }
    }

    Ok(combined)
}
