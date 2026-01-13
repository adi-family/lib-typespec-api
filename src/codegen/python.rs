//! Python Code Generator

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
    package_name: &str,
    side: Side,
) -> Result<Vec<String>, CodegenError> {
    let mut generated = Vec::new();
    let scalars = build_scalar_map(file);
    let models = build_model_map(file);

    fs::create_dir_all(output_dir)?;

    // Generate models
    let models_content = generate_models(file, &scalars, &models)?;
    let models_path = output_dir.join("models.py");
    fs::write(&models_path, models_content)?;
    generated.push(models_path.display().to_string());

    // Generate enums
    let enums_content = generate_enums(file)?;
    let enums_path = output_dir.join("enums.py");
    fs::write(&enums_path, enums_content)?;
    generated.push(enums_path.display().to_string());

    // Generate client
    if matches!(side, Side::Client | Side::Both) {
        let client_dir = output_dir.join("client");
        fs::create_dir_all(&client_dir)?;

        let client_content = generate_client(file, &scalars)?;
        let client_path = client_dir.join("__init__.py");
        fs::write(&client_path, client_content)?;
        generated.push(client_path.display().to_string());
    }

    // Generate server
    if matches!(side, Side::Server | Side::Both) {
        let server_dir = output_dir.join("server");
        fs::create_dir_all(&server_dir)?;

        let server_content = generate_server(file, &scalars)?;
        let server_path = server_dir.join("__init__.py");
        fs::write(&server_path, server_content)?;
        generated.push(server_path.display().to_string());
    }

    // Generate __init__.py
    let init_content = generate_init(package_name)?;
    let init_path = output_dir.join("__init__.py");
    fs::write(&init_path, init_content)?;
    generated.push(init_path.display().to_string());

    Ok(generated)
}

fn generate_models(
    file: &TypeSpecFile,
    scalars: &ScalarMap,
    models: &ModelMap<'_>,
) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(
        out,
        r#""""
Auto-generated models from TypeSpec.
DO NOT EDIT.
""""#
    )?;
    writeln!(out)?;
    writeln!(out, "from __future__ import annotations")?;
    writeln!(out, "from dataclasses import dataclass, field")?;
    writeln!(out, "from datetime import datetime")?;
    writeln!(
        out,
        "from typing import Any, Optional, List, Dict, Literal, TypeVar, Generic"
    )?;
    writeln!(out, "from uuid import UUID")?;
    writeln!(out)?;
    writeln!(out, "T = TypeVar('T')")?;
    writeln!(out)?;

    for model in file.models() {
        writeln!(out)?;
        writeln!(out, "@dataclass")?;
        // Add Generic base if model has type parameters
        if model.type_params.is_empty() {
            writeln!(out, "class {}:", model.name)?;
        } else {
            let params = model.type_params.join(", ");
            writeln!(out, "class {}(Generic[{}]):", model.name, params)?;
        }

        if let Some(desc) = get_description(&model.decorators) {
            writeln!(out, r#"    """{}""""#, desc)?;
        }

        // Resolve spread references and get all properties
        let all_properties = resolve_properties(model, models);

        if all_properties.is_empty() {
            writeln!(out, "    pass")?;
        } else {
            // Required fields first
            for prop in all_properties.iter().filter(|p| !p.optional) {
                let py_type = type_to_python(&prop.type_ref, scalars);
                let name = prop.name.to_case(Case::Snake);
                writeln!(out, "    {}: {}", name, py_type)?;
            }

            // Optional fields
            for prop in all_properties.iter().filter(|p| p.optional) {
                let py_type = type_to_python(&prop.type_ref, scalars);
                let name = prop.name.to_case(Case::Snake);
                writeln!(out, "    {}: Optional[{}] = None", name, py_type)?;
            }
        }

        // Add to_dict method
        writeln!(out)?;
        writeln!(out, "    def to_dict(self) -> Dict[str, Any]:")?;
        writeln!(out, "        result: Dict[str, Any] = {{}}")?;
        for prop in &all_properties {
            let name = prop.name.to_case(Case::Snake);
            let orig = &prop.name;
            if prop.optional {
                writeln!(out, "        if self.{} is not None:", name)?;
                writeln!(out, r#"            result["{}"] = self.{}"#, orig, name)?;
            } else {
                writeln!(out, r#"        result["{}"] = self.{}"#, orig, name)?;
            }
        }
        writeln!(out, "        return result")?;

        // Add from_dict method
        writeln!(out)?;
        writeln!(out, "    @classmethod")?;
        writeln!(
            out,
            "    def from_dict(cls, data: Dict[str, Any]) -> \"{}\":",
            model.name
        )?;
        writeln!(out, "        return cls(")?;
        for prop in &all_properties {
            let name = prop.name.to_case(Case::Snake);
            let orig = &prop.name;
            writeln!(out, r#"            {}=data.get("{}"),"#, name, orig)?;
        }
        writeln!(out, "        )")?;
    }

    Ok(out)
}

fn generate_enums(file: &TypeSpecFile) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(
        out,
        r#""""
Auto-generated enums from TypeSpec.
DO NOT EDIT.
""""#
    )?;
    writeln!(out)?;
    writeln!(out, "from enum import Enum")?;
    writeln!(out)?;

    for enum_def in file.enums() {
        writeln!(out)?;
        writeln!(out, "class {}(str, Enum):", enum_def.name)?;

        for member in &enum_def.members {
            let value = member
                .value
                .as_ref()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    _ => member.name.to_case(Case::Snake),
                })
                .unwrap_or_else(|| member.name.to_case(Case::Snake));

            let variant = member.name.to_case(Case::ScreamingSnake);
            writeln!(out, r#"    {} = "{}""#, variant, value)?;
        }
    }

    Ok(out)
}

fn generate_client(file: &TypeSpecFile, scalars: &ScalarMap) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(
        out,
        r#""""
Auto-generated API client from TypeSpec.
DO NOT EDIT.
""""#
    )?;
    writeln!(out)?;
    writeln!(out, "from __future__ import annotations")?;
    writeln!(out, "import httpx")?;
    writeln!(out, "from typing import Any, Optional, Dict")?;
    writeln!(out, "from ..models import *")?;
    writeln!(out, "from ..enums import *")?;
    writeln!(out)?;

    // Base client
    writeln!(
        out,
        r#"
class ApiError(Exception):
    def __init__(self, status_code: int, message: str):
        self.status_code = status_code
        self.message = message
        super().__init__(f"{{status_code}}: {{message}}")


class BaseClient:
    def __init__(self, base_url: str, access_token: Optional[str] = None):
        self.base_url = base_url.rstrip("/")
        self.access_token = access_token
        self._client: Optional[httpx.AsyncClient] = None

    async def __aenter__(self):
        self._client = httpx.AsyncClient()
        return self

    async def __aexit__(self, *args):
        if self._client:
            await self._client.aclose()

    def _headers(self) -> Dict[str, str]:
        headers = {{"Content-Type": "application/json"}}
        if self.access_token:
            headers["Authorization"] = f"Bearer {{self.access_token}}"
        return headers

    async def _request(self, method: str, path: str, **kwargs) -> Any:
        url = f"{{self.base_url}}{{path}}"
        resp = await self._client.request(method, url, headers=self._headers(), **kwargs)
        if resp.status_code >= 400:
            raise ApiError(resp.status_code, resp.text)
        if resp.status_code == 204:
            return None
        return resp.json()
"#
    )?;

    // Service clients
    for iface in file.interfaces() {
        let base_path = get_route(&iface.decorators).unwrap_or_default();
        let class_name = format!("{}Client", iface.name);

        writeln!(out)?;
        writeln!(out, "class {}:", class_name)?;
        writeln!(out, "    def __init__(self, client: BaseClient):")?;
        writeln!(out, "        self._client = client")?;

        for op in &iface.operations {
            let method = get_http_method(&op.decorators);
            let op_path = get_route(&op.decorators).unwrap_or_default();
            let full_path = format!("{}{}", base_path, op_path);

            writeln!(out)?;
            write!(out, "    async def {}(self", op.name.to_case(Case::Snake))?;

            // Parameters
            for param in &op.params {
                if has_decorator(&param.decorators, "path") {
                    write!(out, ", {}: str", param.name.to_case(Case::Snake))?;
                } else if has_decorator(&param.decorators, "body") {
                    let ty = type_to_python(&param.type_ref, scalars);
                    write!(out, ", body: {}", ty)?;
                } else if has_decorator(&param.decorators, "query") {
                    let ty = type_to_python(&param.type_ref, scalars);
                    if param.optional {
                        write!(
                            out,
                            ", {}: Optional[{}] = None",
                            param.name.to_case(Case::Snake),
                            ty
                        )?;
                    } else {
                        write!(out, ", {}: {}", param.name.to_case(Case::Snake), ty)?;
                    }
                }
            }

            // Extract the actual return type, handling response wrappers
            let (return_type, _) = extract_return_type(&op.return_type, scalars);

            writeln!(out, ") -> {}:", return_type)?;

            // Build path with substitutions
            let mut path_code = format!(r#"        path = "{}""#, full_path);
            for param in &op.params {
                if has_decorator(&param.decorators, "path") {
                    let name = param.name.to_case(Case::Snake);
                    path_code = format!(
                        r#"{}
        path = path.replace("{{{{{}}}}}", str({}))"#,
                        path_code, param.name, name
                    );
                }
            }
            writeln!(out, "{}", path_code)?;

            // Build query params
            let query_params: Vec<_> = op
                .params
                .iter()
                .filter(|p| has_decorator(&p.decorators, "query"))
                .collect();

            if !query_params.is_empty() {
                writeln!(out, "        params = {{}}")?;
                for param in &query_params {
                    let name = param.name.to_case(Case::Snake);
                    writeln!(out, "        if {} is not None:", name)?;
                    writeln!(out, r#"            params["{}"] = {}"#, param.name, name)?;
                }
            }

            // Make request
            let has_body = op
                .params
                .iter()
                .any(|p| has_decorator(&p.decorators, "body"));

            write!(
                out,
                "        result = await self._client._request(\"{}\", path",
                method
            )?;
            if has_body {
                write!(out, ", json=body.to_dict()")?;
            }
            if !query_params.is_empty() {
                write!(out, ", params=params")?;
            }
            writeln!(out, ")")?;

            // Return - extract actual body type from response wrapper
            let (_, body_type) = extract_return_type(&op.return_type, scalars);
            if let Some(ty) = body_type {
                if ty == "None" {
                    writeln!(out, "        return None")?;
                } else if is_primitive_type(&ty) {
                    writeln!(out, "        return result")?;
                } else if ty.starts_with("List[") {
                    // Extract inner type from List[X]
                    let inner = &ty[5..ty.len() - 1];
                    if is_primitive_type(inner) {
                        writeln!(out, "        return result")?;
                    } else {
                        writeln!(
                            out,
                            "        return [{}.from_dict(item) for item in result]",
                            inner
                        )?;
                    }
                } else {
                    writeln!(out, "        return {}.from_dict(result)", ty)?;
                }
            } else {
                writeln!(out, "        return result")?;
            }
        }
    }

    // Main client class
    writeln!(out)?;
    writeln!(out, "class Client(BaseClient):")?;
    writeln!(out, "    def __init__(self, *args, **kwargs):")?;
    writeln!(out, "        super().__init__(*args, **kwargs)")?;

    for iface in file.interfaces() {
        let name = iface.name.to_case(Case::Snake);
        let class_name = format!("{}Client", iface.name);
        writeln!(out, "        self.{} = {}(self)", name, class_name)?;
    }

    Ok(out)
}

fn generate_server(file: &TypeSpecFile, scalars: &ScalarMap) -> Result<String, CodegenError> {
    let mut out = String::new();

    writeln!(
        out,
        r#""""
Auto-generated server handlers from TypeSpec.
DO NOT EDIT.

Implement the abstract methods in a subclass.
""""#
    )?;
    writeln!(out)?;
    writeln!(out, "from abc import ABC, abstractmethod")?;
    writeln!(out, "from typing import Any, Optional")?;
    writeln!(out, "from ..models import *")?;
    writeln!(out, "from ..enums import *")?;
    writeln!(out)?;

    for iface in file.interfaces() {
        writeln!(out)?;
        writeln!(out, "class {}Handler(ABC):", iface.name)?;

        for op in &iface.operations {
            writeln!(out)?;
            writeln!(out, "    @abstractmethod")?;
            write!(out, "    async def {}(self", op.name.to_case(Case::Snake))?;

            for param in &op.params {
                let name = param.name.to_case(Case::Snake);
                let ty = type_to_python(&param.type_ref, scalars);
                if param.optional {
                    write!(out, ", {}: Optional[{}]", name, ty)?;
                } else {
                    write!(out, ", {}: {}", name, ty)?;
                }
            }

            let return_type = op
                .return_type
                .as_ref()
                .map(|t| type_to_python(t, scalars))
                .unwrap_or_else(|| "None".to_string());

            writeln!(out, ") -> {}:", return_type)?;
            writeln!(out, "        raise NotImplementedError")?;
        }
    }

    Ok(out)
}

fn generate_init(_package_name: &str) -> Result<String, CodegenError> {
    Ok(r#""""
Auto-generated from TypeSpec.
""""

from .models import *
from .enums import *
from .client import Client, ApiError
"#
    .to_string())
}

/// Convert TypeSpec type to Python type string
pub fn type_to_python(type_ref: &TypeRef, scalars: &ScalarMap) -> String {
    match type_ref {
        TypeRef::Builtin(name) => builtin_to_python(name),
        TypeRef::Named(name) => {
            // Check if this is a custom scalar type
            if let Some(base_type) = scalars.get(name) {
                builtin_to_python(base_type)
            } else {
                name.clone()
            }
        }
        TypeRef::Qualified(parts) => parts.last().cloned().unwrap_or_default(),
        TypeRef::Array(inner) => format!("List[{}]", type_to_python(inner, scalars)),
        TypeRef::Generic { base, args } => {
            let base_name = type_to_python(base, scalars);
            // Handle Record<T> -> Dict[str, T]
            if base_name == "Record" && args.len() == 1 {
                return format!("Dict[str, {}]", type_to_python(&args[0], scalars));
            }
            let args_str: Vec<_> = args.iter().map(|a| type_to_python(a, scalars)).collect();
            format!("{}[{}]", base_name, args_str.join(", "))
        }
        TypeRef::Optional(inner) => format!("Optional[{}]", type_to_python(inner, scalars)),
        TypeRef::Union(variants) => {
            // Check if all variants are string literals -> use Literal
            let all_string_literals = variants
                .iter()
                .all(|v| matches!(v, TypeRef::StringLiteral(_)));

            if all_string_literals {
                let literals: Vec<_> = variants
                    .iter()
                    .filter_map(|v| {
                        if let TypeRef::StringLiteral(s) = v {
                            Some(format!("\"{}\"", s))
                        } else {
                            None
                        }
                    })
                    .collect();
                format!("Literal[{}]", literals.join(", "))
            } else {
                let types: Vec<_> = variants
                    .iter()
                    .map(|v| type_to_python(v, scalars))
                    .collect();
                types.join(" | ")
            }
        }
        TypeRef::StringLiteral(s) => format!("Literal[\"{}\"]", s),
        TypeRef::IntLiteral(n) => format!("Literal[{}]", n),
        _ => "Any".to_string(),
    }
}

/// Convert builtin TypeSpec type to Python
fn builtin_to_python(name: &str) -> String {
    match name {
        "string" => "str".to_string(),
        "int8" | "int16" | "int32" | "int64" | "uint8" | "uint16" | "uint32" | "uint64" => {
            "int".to_string()
        }
        "float32" | "float64" => "float".to_string(),
        "boolean" => "bool".to_string(),
        "utcDateTime" | "offsetDateTime" | "plainDate" | "plainTime" => "datetime".to_string(),
        "bytes" => "bytes".to_string(),
        "void" | "null" => "None".to_string(),
        _ => "Any".to_string(),
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

/// Extract return type from response wrappers like `{ @statusCode: 200; @body body: T } | ApiError`
/// Returns (display_type, body_type) where body_type is the actual type to deserialize
fn extract_return_type(
    type_ref: &Option<TypeRef>,
    scalars: &ScalarMap,
) -> (String, Option<String>) {
    match type_ref {
        None => ("None".to_string(), None),
        Some(TypeRef::Union(variants)) => {
            // Look for response wrapper with @body
            for variant in variants {
                if let TypeRef::AnonymousModel(props) = variant {
                    // Find @body property
                    for prop in props {
                        if has_decorator(&prop.decorators, "body") {
                            let body_type = type_to_python(&prop.type_ref, scalars);
                            return (body_type.clone(), Some(body_type));
                        }
                    }
                    // Check for statusCode: 204 (no content)
                    for prop in props {
                        if has_decorator(&prop.decorators, "statusCode") {
                            if let Some(Value::Int(204)) = &prop.default {
                                return ("None".to_string(), Some("None".to_string()));
                            }
                            if let TypeRef::IntLiteral(204) = &prop.type_ref {
                                return ("None".to_string(), Some("None".to_string()));
                            }
                        }
                    }
                }
            }
            // No body found, return generic type
            let types: Vec<_> = variants
                .iter()
                .map(|v| type_to_python(v, scalars))
                .collect();
            let combined = types.join(" | ");
            (combined.clone(), None)
        }
        Some(TypeRef::AnonymousModel(props)) => {
            // Single response wrapper
            for prop in props {
                if has_decorator(&prop.decorators, "body") {
                    let body_type = type_to_python(&prop.type_ref, scalars);
                    return (body_type.clone(), Some(body_type));
                }
            }
            ("Any".to_string(), None)
        }
        Some(tr) => {
            let ty = type_to_python(tr, scalars);
            (ty.clone(), Some(ty))
        }
    }
}

/// Check if a Python type is primitive (doesn't need from_dict)
fn is_primitive_type(ty: &str) -> bool {
    matches!(
        ty,
        "str" | "int" | "float" | "bool" | "bytes" | "None" | "Any" | "datetime" | "Dict[str, Any]"
    ) || ty.starts_with("Literal[")
        || ty.starts_with("Dict[")
}
