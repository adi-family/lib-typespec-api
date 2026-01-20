//! TypeScript Code Generator

use crate::ast::*;
use crate::codegen::{
    build_model_map, build_scalar_map, resolve_properties, CodegenError, ModelMap, ScalarMap, Side,
};
use convert_case::{Case, Casing};
use std::fmt::Write;
use std::fs;
use std::path::Path;

pub fn generate(
    file: &TypeSpecFile,
    output_dir: &Path,
    _package_name: &str,
    side: Side,
) -> Result<Vec<String>, CodegenError> {
    let mut generated = Vec::new();
    let scalars = build_scalar_map(file);
    let models = build_model_map(file);

    fs::create_dir_all(output_dir)?;

    // Generate models
    let models_content = generate_models(file, &scalars, &models)?;
    let models_path = output_dir.join("models.ts");
    fs::write(&models_path, models_content)?;
    generated.push(models_path.display().to_string());

    // Generate enums
    let enums_content = generate_enums(file)?;
    let enums_path = output_dir.join("enums.ts");
    fs::write(&enums_path, enums_content)?;
    generated.push(enums_path.display().to_string());

    // Generate client
    if matches!(side, Side::Client | Side::Both) {
        let client_content = generate_client(file)?;
        let client_path = output_dir.join("client.ts");
        fs::write(&client_path, client_content)?;
        generated.push(client_path.display().to_string());
    }

    // Generate server
    if matches!(side, Side::Server | Side::Both) {
        let server_content = generate_server(file)?;
        let server_path = output_dir.join("server.ts");
        fs::write(&server_path, server_content)?;
        generated.push(server_path.display().to_string());
    }

    // Generate index
    let index_content = generate_index(side)?;
    let index_path = output_dir.join("index.ts");
    fs::write(&index_path, index_content)?;
    generated.push(index_path.display().to_string());

    Ok(generated)
}

fn generate_models(
    file: &TypeSpecFile,
    _scalars: &ScalarMap,
    models: &ModelMap<'_>,
) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated models from TypeSpec.")?;
    writeln!(out, " * DO NOT EDIT.")?;
    writeln!(out, " */")?;
    writeln!(out)?;

    for model in file.models() {
        writeln!(out)?;
        if let Some(desc) = get_description(&model.decorators) {
            writeln!(out, "/** {} */", desc)?;
        }

        // Add type parameters if present
        let type_params = if model.type_params.is_empty() {
            String::new()
        } else {
            format!("<{}>", model.type_params.join(", "))
        };
        writeln!(out, "export interface {}{} {{", model.name, type_params)?;

        // Resolve spread references and get all properties
        let all_properties = resolve_properties(model, models);

        for prop in all_properties {
            let ts_type = type_to_typescript(&prop.type_ref);
            let optional = if prop.optional { "?" } else { "" };
            writeln!(out, "  {}{}: {};", prop.name, optional, ts_type)?;
        }

        writeln!(out, "}}")?;
    }

    Ok(out)
}

fn generate_enums(file: &TypeSpecFile) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated enums from TypeSpec.")?;
    writeln!(out, " * DO NOT EDIT.")?;
    writeln!(out, " */")?;
    writeln!(out)?;

    for enum_def in file.enums() {
        writeln!(out)?;
        writeln!(out, "export enum {} {{", enum_def.name)?;

        for member in &enum_def.members {
            let value = member
                .value
                .as_ref()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    _ => member.name.to_case(Case::Snake),
                })
                .unwrap_or_else(|| member.name.to_case(Case::Snake));

            let variant = member.name.to_case(Case::Pascal);
            writeln!(out, r#"  {} = "{}","#, variant, value)?;
        }

        writeln!(out, "}}")?;
    }

    Ok(out)
}

fn generate_client(file: &TypeSpecFile) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated API client from TypeSpec.")?;
    writeln!(out, " * DO NOT EDIT.")?;
    writeln!(out, " */")?;
    writeln!(out)?;

    // Collect all model and enum names for imports
    let model_names: Vec<_> = file.models().map(|m| m.name.as_str()).collect();
    let enum_names: Vec<_> = file.enums().map(|e| e.name.as_str()).collect();

    if !model_names.is_empty() {
        writeln!(
            out,
            "import type {{ {} }} from './models';",
            model_names.join(", ")
        )?;
    }
    if !enum_names.is_empty() {
        writeln!(
            out,
            "import {{ {} }} from './enums';",
            enum_names.join(", ")
        )?;
    }
    writeln!(out)?;

    // Base client
    writeln!(
        out,
        r#"
export class ApiError extends Error {{
  constructor(
    public statusCode: number,
    public code: string,
    message: string
  ) {{
    super(message);
  }}
}}

export interface ClientConfig {{
  baseUrl: string;
  accessToken?: string;
  fetch?: typeof fetch;
}}

export class BaseClient {{
  private baseUrl: string;
  private accessToken?: string;
  private fetchFn: typeof fetch;

  constructor(config: ClientConfig) {{
    this.baseUrl = config.baseUrl.replace(/\/$/, '');
    this.accessToken = config.accessToken;
    this.fetchFn = config.fetch ?? fetch;
  }}

  setAccessToken(token: string) {{
    this.accessToken = token;
  }}

  protected async request<T>(
    method: string,
    path: string,
    options: {{ body?: unknown; query?: Record<string, unknown> }} = {{}}
  ): Promise<T> {{
    const url = new URL(path, this.baseUrl);
    if (options.query) {{
      for (const [k, v] of Object.entries(options.query)) {{
        if (v !== undefined) url.searchParams.set(k, String(v));
      }}
    }}

    const headers: Record<string, string> = {{ 'Content-Type': 'application/json' }};
    if (this.accessToken) {{
      headers['Authorization'] = `Bearer ${{this.accessToken}}`;
    }}

    const resp = await this.fetchFn(url.toString(), {{
      method,
      headers,
      body: options.body ? JSON.stringify(options.body) : undefined,
    }});

    if (!resp.ok) {{
      const err = await resp.json().catch(() => ({{}}));
      throw new ApiError(resp.status, err.code ?? 'ERROR', err.message ?? resp.statusText);
    }}

    if (resp.status === 204) return undefined as T;
    return resp.json();
  }}
}}
"#
    )?;

    // Service clients
    for iface in file.interfaces() {
        let base_path = get_route(&iface.decorators).unwrap_or_default();
        let class_name = format!("{}Client", iface.name);

        writeln!(out)?;
        writeln!(out, "export class {} extends BaseClient {{", class_name)?;

        for op in &iface.operations {
            let method = get_http_method(&op.decorators);
            let op_path = get_route(&op.decorators).unwrap_or_default();
            let full_path = format!("{}{}", base_path, op_path);

            writeln!(out)?;
            write!(out, "  async {}(", op.name.to_case(Case::Camel))?;

            // Parameters
            let mut params = Vec::new();
            for param in &op.params {
                let name = param.name.to_case(Case::Camel);
                let ty = type_to_typescript(&param.type_ref);
                let optional = if param.optional { "?" } else { "" };
                params.push(format!("{}{}: {}", name, optional, ty));
            }
            write!(out, "{}", params.join(", "))?;

            let return_type = op
                .return_type
                .as_ref()
                .map(|t| type_to_typescript(t))
                .unwrap_or_else(|| "void".to_string());

            writeln!(out, "): Promise<{}> {{", return_type)?;

            // Build path
            let mut path_expr = format!("`{}`", full_path);
            for param in &op.params {
                if has_decorator(&param.decorators, "path") {
                    let name = param.name.to_case(Case::Camel);
                    path_expr = path_expr
                        .replace(&format!("{{{}}}", param.name), &format!("${{{}}}", name));
                }
            }
            writeln!(out, "    const path = {};", path_expr)?;

            // Query params
            let query_params: Vec<_> = op
                .params
                .iter()
                .filter(|p| has_decorator(&p.decorators, "query"))
                .collect();

            // Body param
            let body_param = op
                .params
                .iter()
                .find(|p| has_decorator(&p.decorators, "body"));

            write!(out, "    return this.request('{}', path", method)?;

            if body_param.is_some() || !query_params.is_empty() {
                write!(out, ", {{")?;
                if let Some(bp) = body_param {
                    write!(out, " body: {}", bp.name.to_case(Case::Camel))?;
                }
                if !query_params.is_empty() {
                    if body_param.is_some() {
                        write!(out, ",")?;
                    }
                    write!(out, " query: {{ ")?;
                    let qp_strs: Vec<_> = query_params
                        .iter()
                        .map(|p| p.name.to_case(Case::Camel))
                        .collect();
                    write!(out, "{}", qp_strs.join(", "))?;
                    write!(out, " }}")?;
                }
                write!(out, " }}")?;
            }

            writeln!(out, ");")?;
            writeln!(out, "  }}")?;
        }

        writeln!(out, "}}")?;
    }

    // Main client
    writeln!(out)?;
    writeln!(out, "export class Client extends BaseClient {{")?;

    for iface in file.interfaces() {
        let name = iface.name.to_case(Case::Camel);
        let class_name = format!("{}Client", iface.name);
        writeln!(out, "  readonly {}: {};", name, class_name)?;
    }

    writeln!(out)?;
    writeln!(out, "  constructor(config: ClientConfig) {{")?;
    writeln!(out, "    super(config);")?;

    for iface in file.interfaces() {
        let name = iface.name.to_case(Case::Camel);
        let class_name = format!("{}Client", iface.name);
        writeln!(out, "    this.{} = new {}(config);", name, class_name)?;
    }

    writeln!(out, "  }}")?;
    writeln!(out, "}}")?;

    Ok(out)
}

fn generate_server(file: &TypeSpecFile) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated server handlers from TypeSpec.")?;
    writeln!(out, " * DO NOT EDIT.")?;
    writeln!(out, " *")?;
    writeln!(out, " * Implement the abstract methods in a subclass.")?;
    writeln!(out, " */")?;
    writeln!(out)?;

    // Collect all model and enum names for imports
    let model_names: Vec<_> = file.models().map(|m| m.name.as_str()).collect();
    let enum_names: Vec<_> = file.enums().map(|e| e.name.as_str()).collect();

    if !model_names.is_empty() {
        writeln!(
            out,
            "import type {{ {} }} from './models';",
            model_names.join(", ")
        )?;
    }
    if !enum_names.is_empty() {
        writeln!(
            out,
            "import {{ {} }} from './enums';",
            enum_names.join(", ")
        )?;
    }
    writeln!(out)?;

    for iface in file.interfaces() {
        writeln!(out)?;
        writeln!(out, "export abstract class {}Handler {{", iface.name)?;

        for op in &iface.operations {
            let return_type = op
                .return_type
                .as_ref()
                .map(|t| type_to_typescript(t))
                .unwrap_or_else(|| "void".to_string());

            writeln!(out)?;
            write!(out, "  abstract {}(", op.name.to_case(Case::Camel))?;

            let params: Vec<_> = op
                .params
                .iter()
                .map(|p| {
                    let name = p.name.to_case(Case::Camel);
                    let ty = type_to_typescript(&p.type_ref);
                    let opt = if p.optional { "?" } else { "" };
                    format!("{}{}: {}", name, opt, ty)
                })
                .collect();

            write!(out, "{}", params.join(", "))?;
            writeln!(out, "): Promise<{}>;", return_type)?;
        }

        writeln!(out, "}}")?;
    }

    Ok(out)
}

fn generate_index(side: Side) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "/**")?;
    writeln!(out, " * Auto-generated from TypeSpec.")?;
    writeln!(out, " */")?;
    writeln!(out)?;
    writeln!(out, "export * from './models';")?;
    writeln!(out, "export * from './enums';")?;

    if matches!(side, Side::Client | Side::Both) {
        writeln!(out, "export * from './client';")?;
    }

    if matches!(side, Side::Server | Side::Both) {
        writeln!(out, "export * from './server';")?;
    }

    Ok(out)
}

pub fn type_to_typescript(type_ref: &TypeRef) -> String {
    match type_ref {
        TypeRef::Builtin(name) => match name.as_str() {
            "string" | "url" => "string".to_string(),
            "int8" | "int16" | "int32" | "int64" | "uint8" | "uint16" | "uint32" | "uint64"
            | "float32" | "float64" => "number".to_string(),
            "boolean" => "boolean".to_string(),
            "utcDateTime" | "offsetDateTime" | "plainDate" | "plainTime" => "string".to_string(),
            "bytes" => "Uint8Array".to_string(),
            "void" | "null" => "void".to_string(),
            _ => "unknown".to_string(),
        },
        TypeRef::Named(name) => {
            // Handle custom scalar types
            match name.as_str() {
                "uuid" => "string".to_string(),  // uuid scalar extends string
                "email" => "string".to_string(), // email scalar extends string
                "url" => "string".to_string(),   // url scalar extends string
                _ => name.clone(),               // Don't add Models. prefix, types are local
            }
        }
        TypeRef::Qualified(parts) => format!("Models.{}", parts.last().unwrap_or(&String::new())),
        TypeRef::Array(inner) => format!("{}[]", type_to_typescript(inner)),
        TypeRef::Generic { base, args } => {
            let base_name = type_to_typescript(base);
            // Handle Record<T> -> Record<string, T>
            if base_name == "Record" && args.len() == 1 {
                return format!("Record<string, {}>", type_to_typescript(&args[0]));
            }
            let args_str: Vec<_> = args.iter().map(type_to_typescript).collect();
            format!("{}<{}>", base_name, args_str.join(", "))
        }
        TypeRef::Optional(inner) => format!("{} | undefined", type_to_typescript(inner)),
        TypeRef::Union(variants) => {
            let types: Vec<_> = variants.iter().map(type_to_typescript).collect();
            types.join(" | ")
        }
        TypeRef::StringLiteral(s) => format!("'{}'", s),
        _ => "unknown".to_string(),
    }
}

fn get_description(decorators: &[Decorator]) -> Option<String> {
    decorators
        .iter()
        .find(|d| d.name == "doc")
        .and_then(|d| d.get_string_arg(0).map(|s| s.to_string()))
}

fn get_route(decorators: &[Decorator]) -> Option<String> {
    decorators
        .iter()
        .find(|d| d.name == "route")
        .and_then(|d| d.get_string_arg(0).map(|s| s.to_string()))
}

fn get_http_method(decorators: &[Decorator]) -> &'static str {
    for d in decorators {
        match d.name.as_str() {
            "get" => return "GET",
            "post" => return "POST",
            "put" => return "PUT",
            "patch" => return "PATCH",
            "delete" => return "DELETE",
            _ => {}
        }
    }
    "GET"
}

fn has_decorator(decorators: &[Decorator], name: &str) -> bool {
    decorators.iter().any(|d| d.name == name)
}
