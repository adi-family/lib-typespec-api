//! OpenAPI 3.0 Schema Generator
//!
//! Generates OpenAPI 3.0 specification from TypeSpec AST.

use crate::ast::*;
use crate::codegen::{build_model_map, build_scalar_map, resolve_properties, CodegenError, ModelMap, ScalarMap};
use convert_case::{Case, Casing};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::Path;

pub fn generate(
    file: &TypeSpecFile,
    output_dir: &Path,
    title: &str,
) -> Result<Vec<String>, CodegenError> {
    let mut generated = Vec::new();
    let scalars = build_scalar_map(file);
    let models = build_model_map(file);

    fs::create_dir_all(output_dir)?;

    let spec = generate_openapi_spec(file, &scalars, &models, title)?;

    // Write JSON
    let json_path = output_dir.join("openapi.json");
    let json_content = serde_json::to_string_pretty(&spec)
        .map_err(|e| CodegenError::Generation(e.to_string()))?;
    fs::write(&json_path, json_content)?;
    generated.push(json_path.display().to_string());

    // Write YAML (simple conversion)
    let yaml_path = output_dir.join("openapi.yaml");
    let yaml_content = json_to_yaml(&spec);
    fs::write(&yaml_path, yaml_content)?;
    generated.push(yaml_path.display().to_string());

    Ok(generated)
}

fn generate_openapi_spec(
    file: &TypeSpecFile,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
    title: &str,
) -> Result<Value, CodegenError> {
    let mut spec = json!({
        "openapi": "3.0.3",
        "info": {
            "title": title,
            "version": "1.0.0"
        },
        "paths": {},
        "components": {
            "schemas": {},
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            }
        },
        "security": [{ "bearerAuth": [] }]
    });

    // Generate schemas for models
    let schemas = spec["components"]["schemas"].as_object_mut().unwrap();
    for model in file.models() {
        let schema = model_to_schema(model, scalars, models);
        schemas.insert(model.name.clone(), schema);
    }

    // Generate schemas for enums
    for enum_def in file.enums() {
        let schema = enum_to_schema(enum_def);
        schemas.insert(enum_def.name.clone(), schema);
    }

    // Generate paths from interfaces
    let paths = spec["paths"].as_object_mut().unwrap();
    for iface in file.interfaces() {
        let base_path = get_route(&iface.decorators).unwrap_or_default();

        for op in &iface.operations {
            let op_path = get_route(&op.decorators).unwrap_or_default();
            let full_path = format!("{}{}", base_path, op_path);
            let method = get_http_method(&op.decorators).to_lowercase();

            let operation = operation_to_openapi(op, &iface.name, scalars);

            // Get or create path item
            let path_item = paths
                .entry(full_path)
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .unwrap();

            path_item.insert(method, operation);
        }
    }

    Ok(spec)
}

fn model_to_schema(model: &Model, scalars: &ScalarMap, models: &ModelMap<'_>) -> Value {
    let all_properties = resolve_properties(model, models);

    let mut properties = Map::new();
    let mut required = Vec::new();

    for prop in all_properties {
        let schema = type_to_schema(&prop.type_ref, scalars);
        properties.insert(prop.name.clone(), schema);

        if !prop.optional {
            required.push(Value::String(prop.name.clone()));
        }
    }

    let mut schema = json!({
        "type": "object",
        "properties": properties
    });

    if !required.is_empty() {
        schema["required"] = Value::Array(required);
    }

    if let Some(desc) = get_description(&model.decorators) {
        schema["description"] = Value::String(desc);
    }

    schema
}

fn enum_to_schema(enum_def: &Enum) -> Value {
    let values: Vec<Value> = enum_def
        .members
        .iter()
        .map(|m| {
            m.value
                .as_ref()
                .map(|v| match v {
                    crate::ast::Value::String(s) => Value::String(s.clone()),
                    crate::ast::Value::Int(n) => Value::Number((*n).into()),
                    _ => Value::String(m.name.to_case(Case::Snake)),
                })
                .unwrap_or_else(|| Value::String(m.name.to_case(Case::Snake)))
        })
        .collect();

    json!({
        "type": "string",
        "enum": values
    })
}

fn type_to_schema(type_ref: &TypeRef, scalars: &ScalarMap) -> Value {
    match type_ref {
        TypeRef::Builtin(name) => builtin_to_schema(name),
        TypeRef::Named(name) => {
            // Check if this is a custom scalar
            if let Some(base_type) = scalars.get(name) {
                builtin_to_schema(base_type)
            } else {
                // Reference to another schema
                json!({ "$ref": format!("#/components/schemas/{}", name) })
            }
        }
        TypeRef::Qualified(parts) => {
            let name = parts.last().cloned().unwrap_or_default();
            json!({ "$ref": format!("#/components/schemas/{}", name) })
        }
        TypeRef::Array(inner) => {
            json!({
                "type": "array",
                "items": type_to_schema(inner, scalars)
            })
        }
        TypeRef::Generic { base, args } => {
            // Handle common generics
            if let TypeRef::Named(name) = base.as_ref() {
                if name == "Record" && args.len() == 1 {
                    return json!({
                        "type": "object",
                        "additionalProperties": type_to_schema(&args[0], scalars)
                    });
                }
            }
            // For other generics, just reference the base
            type_to_schema(base, scalars)
        }
        TypeRef::Optional(inner) => {
            let mut schema = type_to_schema(inner, scalars);
            if let Some(obj) = schema.as_object_mut() {
                obj.insert("nullable".to_string(), Value::Bool(true));
            }
            schema
        }
        TypeRef::Union(variants) => {
            // Check if all string literals
            let all_strings = variants.iter().all(|v| matches!(v, TypeRef::StringLiteral(_)));
            if all_strings {
                let values: Vec<Value> = variants
                    .iter()
                    .filter_map(|v| {
                        if let TypeRef::StringLiteral(s) = v {
                            Some(Value::String(s.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                json!({
                    "type": "string",
                    "enum": values
                })
            } else {
                let schemas: Vec<Value> = variants.iter().map(|v| type_to_schema(v, scalars)).collect();
                json!({ "oneOf": schemas })
            }
        }
        TypeRef::StringLiteral(s) => {
            json!({
                "type": "string",
                "enum": [s]
            })
        }
        TypeRef::IntLiteral(n) => {
            json!({
                "type": "integer",
                "enum": [n]
            })
        }
        TypeRef::AnonymousModel(props) => {
            let mut properties = Map::new();
            let mut required = Vec::new();

            for prop in props {
                properties.insert(prop.name.clone(), type_to_schema(&prop.type_ref, scalars));
                if !prop.optional {
                    required.push(Value::String(prop.name.clone()));
                }
            }

            let mut schema = json!({
                "type": "object",
                "properties": properties
            });

            if !required.is_empty() {
                schema["required"] = Value::Array(required);
            }

            schema
        }
        _ => json!({ "type": "object" }),
    }
}

fn builtin_to_schema(name: &str) -> Value {
    match name {
        "string" => json!({ "type": "string" }),
        "int8" | "int16" | "int32" => json!({ "type": "integer", "format": "int32" }),
        "int64" => json!({ "type": "integer", "format": "int64" }),
        "uint8" | "uint16" | "uint32" => json!({ "type": "integer", "format": "int32", "minimum": 0 }),
        "uint64" => json!({ "type": "integer", "format": "int64", "minimum": 0 }),
        "float32" => json!({ "type": "number", "format": "float" }),
        "float64" => json!({ "type": "number", "format": "double" }),
        "boolean" => json!({ "type": "boolean" }),
        "utcDateTime" | "offsetDateTime" => json!({ "type": "string", "format": "date-time" }),
        "plainDate" => json!({ "type": "string", "format": "date" }),
        "plainTime" => json!({ "type": "string", "format": "time" }),
        "bytes" => json!({ "type": "string", "format": "byte" }),
        "url" => json!({ "type": "string", "format": "uri" }),
        "uuid" => json!({ "type": "string", "format": "uuid" }),
        _ => json!({ "type": "object" }),
    }
}

fn operation_to_openapi(op: &Operation, interface_name: &str, scalars: &ScalarMap) -> Value {
    let mut operation = json!({
        "operationId": format!("{}_{}", interface_name, op.name).to_case(Case::Camel),
        "tags": [interface_name],
        "responses": {}
    });

    // Add description
    if let Some(desc) = get_description(&op.decorators) {
        operation["summary"] = Value::String(desc);
    }

    // Process parameters
    let mut parameters = Vec::new();
    let mut request_body: Option<Value> = None;

    for param in &op.params {
        if param.spread && param.name.is_empty() {
            continue;
        }

        if has_decorator(&param.decorators, "path") {
            parameters.push(json!({
                "name": param.name,
                "in": "path",
                "required": true,
                "schema": type_to_schema(&param.type_ref, scalars)
            }));
        } else if has_decorator(&param.decorators, "query") {
            parameters.push(json!({
                "name": param.name,
                "in": "query",
                "required": !param.optional,
                "schema": type_to_schema(&param.type_ref, scalars)
            }));
        } else if has_decorator(&param.decorators, "body") {
            request_body = Some(json!({
                "required": true,
                "content": {
                    "application/json": {
                        "schema": type_to_schema(&param.type_ref, scalars)
                    }
                }
            }));
        }
    }

    if !parameters.is_empty() {
        operation["parameters"] = Value::Array(parameters);
    }

    if let Some(body) = request_body {
        operation["requestBody"] = body;
    }

    // Process response type
    let responses = operation["responses"].as_object_mut().unwrap();

    if let Some(ret) = &op.return_type {
        let (status_code, body_schema) = extract_response_info(ret, scalars);

        if let Some(schema) = body_schema {
            responses.insert(status_code.clone(), json!({
                "description": "Successful response",
                "content": {
                    "application/json": {
                        "schema": schema
                    }
                }
            }));
        } else {
            responses.insert(status_code, json!({
                "description": "Successful response (no content)"
            }));
        }
    } else {
        responses.insert("200".to_string(), json!({
            "description": "Successful response"
        }));
    }

    // Add error response
    responses.insert("default".to_string(), json!({
        "description": "Error response",
        "content": {
            "application/json": {
                "schema": {
                    "type": "object",
                    "properties": {
                        "code": { "type": "string" },
                        "message": { "type": "string" }
                    }
                }
            }
        }
    }));

    operation
}

fn extract_response_info(type_ref: &TypeRef, scalars: &ScalarMap) -> (String, Option<Value>) {
    match type_ref {
        TypeRef::Union(variants) => {
            for variant in variants {
                if let TypeRef::AnonymousModel(props) = variant {
                    let mut status_code = "200".to_string();
                    let mut body_schema = None;

                    for prop in props {
                        if has_decorator(&prop.decorators, "statusCode") {
                            if let TypeRef::IntLiteral(code) = &prop.type_ref {
                                status_code = code.to_string();
                            }
                        }
                        if has_decorator(&prop.decorators, "body") {
                            body_schema = Some(type_to_schema(&prop.type_ref, scalars));
                        }
                    }

                    if status_code == "204" {
                        return (status_code, None);
                    }
                    if body_schema.is_some() {
                        return (status_code, body_schema);
                    }
                }
            }
            ("200".to_string(), None)
        }
        TypeRef::AnonymousModel(props) => {
            let mut status_code = "200".to_string();
            let mut body_schema = None;

            for prop in props {
                if has_decorator(&prop.decorators, "statusCode") {
                    if let TypeRef::IntLiteral(code) = &prop.type_ref {
                        status_code = code.to_string();
                    }
                }
                if has_decorator(&prop.decorators, "body") {
                    body_schema = Some(type_to_schema(&prop.type_ref, scalars));
                }
            }

            (status_code, body_schema)
        }
        _ => ("200".to_string(), Some(type_to_schema(type_ref, scalars))),
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

/// Simple JSON to YAML conversion (no external dependency)
fn json_to_yaml(value: &Value) -> String {
    let mut out = String::new();
    json_to_yaml_impl(value, &mut out, 0);
    out
}

fn json_to_yaml_impl(value: &Value, out: &mut String, indent: usize) {
    let prefix = "  ".repeat(indent);

    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => {
            if s.contains('\n') || s.contains(':') || s.contains('#') || s.is_empty() {
                out.push_str(&format!("\"{}\"", s.replace('\"', "\\\"")));
            } else {
                out.push_str(s);
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                out.push_str("[]");
            } else {
                for item in arr {
                    out.push('\n');
                    out.push_str(&prefix);
                    out.push_str("- ");
                    if matches!(item, Value::Object(_)) {
                        json_to_yaml_impl(item, out, indent + 1);
                    } else {
                        json_to_yaml_impl(item, out, 0);
                    }
                }
            }
        }
        Value::Object(obj) => {
            let mut first = true;
            for (key, val) in obj {
                if !first {
                    out.push('\n');
                    out.push_str(&prefix);
                }
                first = false;
                out.push_str(key);
                out.push(':');

                match val {
                    Value::Object(_) | Value::Array(_) if !is_empty_compound(val) => {
                        out.push('\n');
                        out.push_str(&"  ".repeat(indent + 1));
                        json_to_yaml_impl(val, out, indent + 1);
                    }
                    _ => {
                        out.push(' ');
                        json_to_yaml_impl(val, out, indent + 1);
                    }
                }
            }
        }
    }
}

fn is_empty_compound(value: &Value) -> bool {
    match value {
        Value::Array(arr) => arr.is_empty(),
        Value::Object(obj) => obj.is_empty(),
        _ => false,
    }
}
