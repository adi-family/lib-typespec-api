//! Code Generators
//!
//! Generate Python, TypeScript, Rust code, and OpenAPI specs from TypeSpec AST.

pub mod openapi;
pub mod python;
pub mod rust;
pub mod typescript;

use crate::ast::{Model, Property, TypeRef, TypeSpecFile};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Map of scalar name -> base type it extends
pub type ScalarMap = HashMap<String, String>;

/// Map of model name -> Model
pub type ModelMap<'a> = HashMap<&'a str, &'a Model>;

/// Build a map of custom scalars from parsed TypeSpec file
pub fn build_scalar_map(file: &TypeSpecFile) -> ScalarMap {
    file.scalars()
        .filter_map(|s| {
            s.extends
                .as_ref()
                .map(|base| (s.name.clone(), base.clone()))
        })
        .collect()
}

/// Build a map of model names to models for spread resolution
pub fn build_model_map(file: &TypeSpecFile) -> ModelMap<'_> {
    file.models().map(|m| (m.name.as_str(), m)).collect()
}

/// Resolve spread references and return all properties including spread ones
pub fn resolve_properties<'a>(model: &'a Model, models: &'a ModelMap<'a>) -> Vec<&'a Property> {
    let mut properties = Vec::new();

    // First add properties from spread references
    for spread_ref in &model.spread_refs {
        if let Some(name) = get_type_name(spread_ref) {
            if let Some(spread_model) = models.get(name.as_str()) {
                // Recursively resolve spread model's properties
                properties.extend(resolve_properties(spread_model, models));
            }
        }
    }

    // Then add this model's own properties (they override spread ones)
    properties.extend(model.properties.iter());

    properties
}

/// Get the type name from a TypeRef
fn get_type_name(type_ref: &TypeRef) -> Option<String> {
    match type_ref {
        TypeRef::Named(name) => Some(name.clone()),
        TypeRef::Qualified(parts) => parts.last().cloned(),
        _ => None,
    }
}

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Format error: {0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("Generation error: {0}")]
    Generation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Language {
    #[value(name = "python")]
    Python,
    #[value(name = "typescript", alias = "ts")]
    TypeScript,
    #[value(name = "rust", alias = "rs")]
    Rust,
    #[value(name = "openapi", alias = "oas")]
    OpenApi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Side {
    Client,
    Server,
    Both,
}

pub struct Generator<'a> {
    file: &'a TypeSpecFile,
    output_dir: &'a Path,
    package_name: &'a str,
}

impl<'a> Generator<'a> {
    pub fn new(file: &'a TypeSpecFile, output_dir: &'a Path, package_name: &'a str) -> Self {
        Self {
            file,
            output_dir,
            package_name,
        }
    }

    pub fn generate(&self, language: Language, side: Side) -> Result<Vec<String>, CodegenError> {
        let mut generated = Vec::new();

        match language {
            Language::Python => {
                generated.extend(python::generate(
                    self.file,
                    self.output_dir,
                    self.package_name,
                    side,
                )?);
            }
            Language::TypeScript => {
                generated.extend(typescript::generate(
                    self.file,
                    self.output_dir,
                    self.package_name,
                    side,
                )?);
            }
            Language::Rust => {
                generated.extend(rust::generate(
                    self.file,
                    self.output_dir,
                    self.package_name,
                    side,
                )?);
            }
            Language::OpenApi => {
                // OpenAPI ignores side parameter - it generates the full spec
                generated.extend(openapi::generate(
                    self.file,
                    self.output_dir,
                    self.package_name,
                )?);
            }
        }

        Ok(generated)
    }
}
