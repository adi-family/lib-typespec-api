//! Rust Code Generator

use crate::ast::*;
use crate::codegen::{build_model_map, build_scalar_map, resolve_properties, CodegenError, ModelMap, ScalarMap, Side};
use convert_case::{Case, Casing};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

/// Context for tracking inline enums that need to be generated
struct CodegenContext {
    /// Map of enum name -> (variants as string literals)
    inline_enums: RefCell<HashMap<String, Vec<String>>>,
}

impl CodegenContext {
    fn new() -> Self {
        Self {
            inline_enums: RefCell::new(HashMap::new()),
        }
    }

    /// Register an inline enum and return its name
    fn register_inline_enum(&self, model_name: &str, prop_name: &str, variants: &[String]) -> String {
        let enum_name = format!("{}{}", model_name, prop_name.to_case(Case::Pascal));
        self.inline_enums.borrow_mut().insert(enum_name.clone(), variants.to_vec());
        enum_name
    }

    /// Get all registered inline enums
    fn get_inline_enums(&self) -> HashMap<String, Vec<String>> {
        self.inline_enums.borrow().clone()
    }
}

pub fn generate(
    file: &TypeSpecFile,
    output_dir: &Path,
    package_name: &str,
    side: Side,
) -> Result<Vec<String>, CodegenError> {
    let mut generated = Vec::new();
    let scalars = build_scalar_map(file);
    let models = build_model_map(file);

    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Generate Cargo.toml
    let cargo_content = generate_cargo_toml(package_name, side)?;
    let cargo_path = output_dir.join("Cargo.toml");
    fs::write(&cargo_path, cargo_content)?;
    generated.push(cargo_path.display().to_string());

    // Generate lib.rs
    let lib_content = generate_lib(side)?;
    let lib_path = src_dir.join("lib.rs");
    fs::write(&lib_path, lib_content)?;
    generated.push(lib_path.display().to_string());

    // Generate models
    let models_content = generate_models(file, &scalars, &models)?;
    let models_path = src_dir.join("models.rs");
    fs::write(&models_path, models_content)?;
    generated.push(models_path.display().to_string());

    // Generate enums
    let enums_content = generate_enums(file)?;
    let enums_path = src_dir.join("enums.rs");
    fs::write(&enums_path, enums_content)?;
    generated.push(enums_path.display().to_string());

    // Generate client
    if matches!(side, Side::Client | Side::Both) {
        let client_content = generate_client(file, &scalars)?;
        let client_path = src_dir.join("client.rs");
        fs::write(&client_path, client_content)?;
        generated.push(client_path.display().to_string());
    }

    // Generate server
    if matches!(side, Side::Server | Side::Both) {
        let server_content = generate_server(file, &scalars)?;
        let server_path = src_dir.join("server.rs");
        fs::write(&server_path, server_content)?;
        generated.push(server_path.display().to_string());
    }

    Ok(generated)
}

fn generate_cargo_toml(package_name: &str, side: Side) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "[package]")?;
    writeln!(out, r#"name = "{}""#, package_name)?;
    writeln!(out, r#"version = "0.1.0""#)?;
    writeln!(out, r#"edition = "2021""#)?;
    writeln!(out)?;
    writeln!(out, "[dependencies]")?;
    writeln!(out, r#"serde = {{ version = "1.0", features = ["derive"] }}"#)?;
    writeln!(out, r#"serde_json = "1.0""#)?;
    writeln!(out, r#"chrono = {{ version = "0.4", features = ["serde"] }}"#)?;
    writeln!(out, r#"uuid = {{ version = "1.0", features = ["serde", "v4"] }}"#)?;
    writeln!(out, r#"thiserror = "2""#)?;

    if matches!(side, Side::Client | Side::Both) {
        writeln!(out, r#"reqwest = {{ version = "0.12", features = ["json"] }}"#)?;
    }

    if matches!(side, Side::Server | Side::Both) {
        writeln!(out, r#"axum = "0.7""#)?;
        writeln!(out, r#"async-trait = "0.1""#)?;
        writeln!(out, r#"tokio = {{ version = "1", features = ["full"] }}"#)?;
    }

    Ok(out)
}

fn generate_lib(side: Side) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "//! Auto-generated from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "pub mod models;")?;
    writeln!(out, "pub mod enums;")?;

    if matches!(side, Side::Client | Side::Both) {
        writeln!(out, "pub mod client;")?;
    }

    if matches!(side, Side::Server | Side::Both) {
        writeln!(out, "pub mod server;")?;
    }

    Ok(out)
}

fn generate_models(file: &TypeSpecFile, scalars: &ScalarMap, models: &ModelMap<'_>) -> Result<String, CodegenError> {
    let mut out = String::new();
    let ctx = CodegenContext::new();

    writeln!(out, "//! Auto-generated models from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "#![allow(unused_imports)]")?;
    writeln!(out)?;
    writeln!(out, "use crate::enums::*;")?;
    writeln!(out, "use chrono::{{DateTime, Utc}};")?;
    writeln!(out, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(out, "use std::collections::HashMap;")?;
    writeln!(out, "use uuid::Uuid;")?;
    writeln!(out)?;

    // First pass: collect all structs and inline enums
    let mut struct_defs = String::new();

    for model in file.models() {
        // Skip generic models - they need special handling
        if !model.type_params.is_empty() {
            write_generic_model(&mut struct_defs, model, scalars, models)?;
            continue;
        }

        writeln!(struct_defs)?;
        if let Some(desc) = get_description(&model.decorators) {
            writeln!(struct_defs, "/// {}", desc)?;
        }
        writeln!(struct_defs, "#[derive(Debug, Clone, Serialize, Deserialize)]")?;
        writeln!(struct_defs, "#[serde(rename_all = \"camelCase\")]")?;
        writeln!(struct_defs, "pub struct {} {{", model.name)?;

        // Resolve spread references and get all properties
        let all_properties = resolve_properties(model, models);

        for prop in all_properties {
            let rust_type = type_to_rust_with_context(&prop.type_ref, prop.optional, scalars, &ctx, &model.name, &prop.name);
            let name = prop.name.to_case(Case::Snake);

            if prop.optional {
                writeln!(struct_defs, "    #[serde(skip_serializing_if = \"Option::is_none\")]")?;
            }

            // Handle name conflicts with Rust keywords
            let field_name = if is_rust_keyword(&name) {
                format!("r#{}", name)
            } else {
                name
            };

            writeln!(struct_defs, "    pub {}: {},", field_name, rust_type)?;
        }

        writeln!(struct_defs, "}}")?;
    }

    // Generate inline enums first
    for (enum_name, variants) in ctx.get_inline_enums() {
        writeln!(out)?;
        writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]")?;
        writeln!(out, "pub enum {} {{", enum_name)?;
        for variant in &variants {
            let variant_name = variant.to_case(Case::Pascal);
            writeln!(out, r#"    #[serde(rename = "{}")]"#, variant)?;
            writeln!(out, "    {},", variant_name)?;
        }
        writeln!(out, "}}")?;
    }

    // Then write struct definitions
    out.push_str(&struct_defs);

    Ok(out)
}

fn write_generic_model(
    out: &mut String,
    model: &Model,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
) -> Result<(), CodegenError> {
    let ctx = CodegenContext::new();

    writeln!(out)?;
    if let Some(desc) = get_description(&model.decorators) {
        writeln!(out, "/// {}", desc)?;
    }
    writeln!(out, "#[derive(Debug, Clone, Serialize, Deserialize)]")?;
    writeln!(out, "#[serde(rename_all = \"camelCase\")]")?;

    // Write struct with type parameters
    let type_params = model.type_params.join(", ");
    writeln!(out, "pub struct {}<{}> {{", model.name, type_params)?;

    let all_properties = resolve_properties(model, models);

    for prop in all_properties {
        let rust_type = type_to_rust_with_context(&prop.type_ref, prop.optional, scalars, &ctx, &model.name, &prop.name);
        let name = prop.name.to_case(Case::Snake);

        if prop.optional {
            writeln!(out, "    #[serde(skip_serializing_if = \"Option::is_none\")]")?;
        }

        let field_name = if is_rust_keyword(&name) {
            format!("r#{}", name)
        } else {
            name
        };

        writeln!(out, "    pub {}: {},", field_name, rust_type)?;
    }

    writeln!(out, "}}")?;
    Ok(())
}

fn generate_enums(file: &TypeSpecFile) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "//! Auto-generated enums from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(out)?;

    for enum_def in file.enums() {
        writeln!(out)?;
        writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]")?;
        writeln!(out, "pub enum {} {{", enum_def.name)?;

        for member in &enum_def.members {
            // Get the serialization value - either explicit or snake_case of name
            let value = member
                .value
                .as_ref()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    _ => member.name.to_case(Case::Snake),
                })
                .unwrap_or_else(|| member.name.to_case(Case::Snake));

            let variant = member.name.to_case(Case::Pascal);

            // Always add rename attribute for explicit serialization
            writeln!(out, r#"    #[serde(rename = "{}")]"#, value)?;
            writeln!(out, "    {},", variant)?;
        }

        writeln!(out, "}}")?;
    }

    Ok(out)
}

fn generate_client(file: &TypeSpecFile, scalars: &ScalarMap) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "//! Auto-generated API client from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out)?;
    writeln!(out, "#![allow(unused_imports)]")?;
    writeln!(out)?;
    writeln!(out, "use crate::models::*;")?;
    writeln!(out, "use crate::enums::*;")?;
    writeln!(out, "use reqwest::{{Client, Method}};")?;
    writeln!(out, "use serde::{{de::DeserializeOwned, Serialize}};")?;
    writeln!(out, "use thiserror::Error;")?;
    writeln!(out, "use uuid::Uuid;")?;
    writeln!(out)?;

    // Error type
    writeln!(out, r#"
#[derive(Debug, Error)]
pub enum ApiError {{
    #[error("HTTP error: {{0}}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {{status}} - {{message}}")]
    Api {{ status: u16, code: String, message: String }},
}}
"#)?;

    // Base client
    writeln!(out, r#"
pub struct BaseClient {{
    client: Client,
    base_url: String,
    access_token: Option<String>,
}}

impl BaseClient {{
    pub fn new(base_url: impl Into<String>) -> Self {{
        Self {{
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            access_token: None,
        }}
    }}

    pub fn with_token(mut self, token: impl Into<String>) -> Self {{
        self.access_token = Some(token.into());
        self
    }}

    pub fn set_token(&mut self, token: impl Into<String>) {{
        self.access_token = Some(token.into());
    }}

    async fn request<T, B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
        B: Serialize,
    {{
        let url = format!("{{}}{{}}", self.base_url, path);
        let mut req = self.client.request(method, &url);

        if let Some(token) = &self.access_token {{
            req = req.header("Authorization", format!("Bearer {{}}", token));
        }}

        if let Some(body) = body {{
            req = req.json(body);
        }}

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {{
            let err: serde_json::Value = resp.json().await.unwrap_or_default();
            return Err(ApiError::Api {{
                status: status.as_u16(),
                code: err["code"].as_str().unwrap_or("ERROR").to_string(),
                message: err["message"].as_str().unwrap_or("").to_string(),
            }});
        }}

        if status == reqwest::StatusCode::NO_CONTENT {{
            return Ok(serde_json::from_value(serde_json::Value::Null).unwrap());
        }}

        Ok(resp.json().await?)
    }}
}}
"#)?;

    // Service clients
    for iface in file.interfaces() {
        let base_path = get_route(&iface.decorators).unwrap_or_default();
        let struct_name = format!("{}Client", iface.name);

        writeln!(out)?;
        writeln!(out, "pub struct {}<'a> {{", struct_name)?;
        writeln!(out, "    client: &'a BaseClient,")?;
        writeln!(out, "}}")?;
        writeln!(out)?;
        writeln!(out, "impl<'a> {}<'a> {{", struct_name)?;
        writeln!(out, "    pub fn new(client: &'a BaseClient) -> Self {{")?;
        writeln!(out, "        Self {{ client }}")?;
        writeln!(out, "    }}")?;

        for op in &iface.operations {
            let method = get_http_method(&op.decorators);
            let op_path = get_route(&op.decorators).unwrap_or_default();
            let full_path = format!("{}{}", base_path, op_path);
            let fn_name = op.name.to_case(Case::Snake);

            writeln!(out)?;
            write!(out, "    pub async fn {}(&self", fn_name)?;

            // Parameters
            for param in &op.params {
                // Skip spread params without explicit names
                if param.spread && param.name.is_empty() {
                    continue;
                }
                let name = param.name.to_case(Case::Snake);
                if has_decorator(&param.decorators, "path") {
                    write!(out, ", {}: &str", name)?;
                } else if has_decorator(&param.decorators, "body") {
                    let ty = type_to_rust(&param.type_ref, false, scalars);
                    write!(out, ", body: &{}", ty)?;
                } else if has_decorator(&param.decorators, "query") {
                    let ty = type_to_rust(&param.type_ref, param.optional, scalars);
                    write!(out, ", {}: {}", name, ty)?;
                }
            }

            let return_type = op
                .return_type
                .as_ref()
                .map(|t| type_to_rust(t, false, scalars))
                .unwrap_or_else(|| "()".to_string());

            writeln!(out, ") -> Result<{}, ApiError> {{", return_type)?;

            // Build path
            let mut path_expr = format!(r#"let path = format!("{}"#, full_path);
            for param in &op.params {
                if has_decorator(&param.decorators, "path") {
                    path_expr = path_expr.replace(&format!("{{{}}}", param.name), "{}");
                }
            }
            let path_args: Vec<_> = op
                .params
                .iter()
                .filter(|p| has_decorator(&p.decorators, "path"))
                .map(|p| p.name.to_case(Case::Snake))
                .collect();

            if path_args.is_empty() {
                writeln!(out, r#"        let path = "{}";"#, full_path)?;
            } else {
                writeln!(out, "{}\"", path_expr)?;
                for arg in &path_args {
                    write!(out, ", {}", arg)?;
                }
                writeln!(out, ");")?;
            }

            // Make request
            let has_body = op.params.iter().any(|p| has_decorator(&p.decorators, "body"));

            writeln!(
                out,
                "        self.client.request(Method::{}, &path, {}).await",
                method,
                if has_body { "Some(body)" } else { "None::<&()>" }
            )?;

            writeln!(out, "    }}")?;
        }

        writeln!(out, "}}")?;
    }

    Ok(out)
}

fn generate_server(file: &TypeSpecFile, scalars: &ScalarMap) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(out, "//! Auto-generated server handlers from TypeSpec.")?;
    writeln!(out, "//! DO NOT EDIT.")?;
    writeln!(out, "//!")?;
    writeln!(out, "//! Implement the trait to provide your business logic.")?;
    writeln!(out)?;
    writeln!(out, "#![allow(unused_imports)]")?;
    writeln!(out)?;
    writeln!(out, "use crate::models::*;")?;
    writeln!(out, "use crate::enums::*;")?;
    writeln!(out, "use async_trait::async_trait;")?;
    writeln!(out, "use axum::{{extract::{{Path, Query, State}}, http::StatusCode, Json, Router}};")?;
    writeln!(out, "use std::sync::Arc;")?;
    writeln!(out, "use uuid::Uuid;")?;
    writeln!(out)?;

    // Error type
    writeln!(out, r#"
#[derive(Debug, serde::Serialize)]
pub struct ApiError {{
    pub status: u16,
    pub code: String,
    pub message: String,
}}

impl axum::response::IntoResponse for ApiError {{
    fn into_response(self) -> axum::response::Response {{
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }}
}}
"#)?;

    // Handler traits
    for iface in file.interfaces() {
        let trait_name = format!("{}Handler", iface.name);

        writeln!(out)?;
        writeln!(out, "#[async_trait]")?;
        writeln!(out, "pub trait {}: Send + Sync + 'static {{", trait_name)?;

        for op in &iface.operations {
            let fn_name = op.name.to_case(Case::Snake);

            write!(out, "    async fn {}(&self", fn_name)?;

            for param in &op.params {
                // Skip spread params without explicit names (they expand into multiple params)
                if param.spread && param.name.is_empty() {
                    continue;
                }
                let name = param.name.to_case(Case::Snake);
                let ty = type_to_rust(&param.type_ref, param.optional, scalars);
                write!(out, ", {}: {}", name, ty)?;
            }

            let return_type = op
                .return_type
                .as_ref()
                .map(|t| type_to_rust(t, false, scalars))
                .unwrap_or_else(|| "()".to_string());

            writeln!(out, ") -> Result<{}, ApiError>;", return_type)?;
        }

        writeln!(out, "}}")?;
    }

    Ok(out)
}

/// Convert TypeSpec type to Rust type string
pub fn type_to_rust(type_ref: &TypeRef, optional: bool, scalars: &ScalarMap) -> String {
    let base = match type_ref {
        TypeRef::Builtin(name) => builtin_to_rust(name),
        TypeRef::Named(name) => {
            // Check if this is a well-known scalar type
            match name.as_str() {
                "uuid" => "Uuid".to_string(),
                "email" | "url" => "String".to_string(),
                _ => {
                    // Check if this is a custom scalar type
                    if let Some(base_type) = scalars.get(name) {
                        builtin_to_rust(base_type)
                    } else {
                        name.clone()
                    }
                }
            }
        }
        TypeRef::Qualified(parts) => parts.last().cloned().unwrap_or_default(),
        TypeRef::Array(inner) => format!("Vec<{}>", type_to_rust(inner, false, scalars)),
        TypeRef::Generic { base, args } => {
            let base_name = type_to_rust(base, false, scalars);
            // Handle Record<T> -> HashMap<String, T>
            if base_name == "Record" && args.len() == 1 {
                format!("std::collections::HashMap<String, {}>", type_to_rust(&args[0], false, scalars))
            } else {
                let args_str: Vec<_> = args.iter().map(|a| type_to_rust(a, false, scalars)).collect();
                format!("{}<{}>", base_name, args_str.join(", "))
            }
        }
        TypeRef::Optional(inner) => format!("Option<{}>", type_to_rust(inner, false, scalars)),
        TypeRef::Union(_) => "serde_json::Value".to_string(),
        _ => "serde_json::Value".to_string(),
    };

    if optional && !matches!(type_ref, TypeRef::Optional(_)) {
        format!("Option<{}>", base)
    } else {
        base
    }
}

/// Convert TypeSpec type to Rust type string with context for inline enum generation
fn type_to_rust_with_context(
    type_ref: &TypeRef,
    optional: bool,
    scalars: &ScalarMap,
    ctx: &CodegenContext,
    model_name: &str,
    prop_name: &str,
) -> String {
    let base = match type_ref {
        TypeRef::Builtin(name) => builtin_to_rust(name),
        TypeRef::Named(name) => {
            match name.as_str() {
                "uuid" => "Uuid".to_string(),
                "email" | "url" => "String".to_string(),
                _ => {
                    if let Some(base_type) = scalars.get(name) {
                        builtin_to_rust(base_type)
                    } else {
                        name.clone()
                    }
                }
            }
        }
        TypeRef::Qualified(parts) => parts.last().cloned().unwrap_or_default(),
        TypeRef::Array(inner) => format!("Vec<{}>", type_to_rust_with_context(inner, false, scalars, ctx, model_name, prop_name)),
        TypeRef::Generic { base, args } => {
            let base_name = type_to_rust_with_context(base, false, scalars, ctx, model_name, prop_name);
            if base_name == "Record" && args.len() == 1 {
                format!("HashMap<String, {}>", type_to_rust_with_context(&args[0], false, scalars, ctx, model_name, prop_name))
            } else {
                let args_str: Vec<_> = args.iter().map(|a| type_to_rust_with_context(a, false, scalars, ctx, model_name, prop_name)).collect();
                format!("{}<{}>", base_name, args_str.join(", "))
            }
        }
        TypeRef::Optional(inner) => format!("Option<{}>", type_to_rust_with_context(inner, false, scalars, ctx, model_name, prop_name)),
        TypeRef::Union(variants) => {
            // Check if all variants are string literals -> generate inline enum
            let string_literals: Vec<String> = variants
                .iter()
                .filter_map(|v| match v {
                    TypeRef::StringLiteral(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();

            if string_literals.len() == variants.len() && !string_literals.is_empty() {
                // All variants are string literals - register inline enum
                ctx.register_inline_enum(model_name, prop_name, &string_literals)
            } else {
                "serde_json::Value".to_string()
            }
        }
        TypeRef::StringLiteral(_) => "String".to_string(),
        TypeRef::IntLiteral(_) => "i64".to_string(),
        _ => "serde_json::Value".to_string(),
    };

    if optional && !matches!(type_ref, TypeRef::Optional(_)) {
        format!("Option<{}>", base)
    } else {
        base
    }
}

/// Convert builtin TypeSpec type to Rust
fn builtin_to_rust(name: &str) -> String {
    match name {
        "string" | "url" => "String".to_string(),
        "int8" => "i8".to_string(),
        "int16" => "i16".to_string(),
        "int32" => "i32".to_string(),
        "int64" => "i64".to_string(),
        "uint8" => "u8".to_string(),
        "uint16" => "u16".to_string(),
        "uint32" => "u32".to_string(),
        "uint64" => "u64".to_string(),
        "float32" => "f32".to_string(),
        "float64" => "f64".to_string(),
        "boolean" => "bool".to_string(),
        "utcDateTime" | "offsetDateTime" => "DateTime<Utc>".to_string(),
        "plainDate" => "chrono::NaiveDate".to_string(),
        "plainTime" => "chrono::NaiveTime".to_string(),
        "bytes" => "Vec<u8>".to_string(),
        "void" | "null" => "()".to_string(),
        _ => "serde_json::Value".to_string(),
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

fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}
