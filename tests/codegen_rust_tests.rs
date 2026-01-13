//! Comprehensive Rust code generation tests

use std::path::Path;
use tempfile::TempDir;
use typespec_api::{
    codegen::{Generator, Language, Side},
    parse,
};

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_rust(source: &str, side: Side) -> (TempDir, Vec<String>) {
    let file = parse(source).expect("Failed to parse TypeSpec");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let generator = Generator::new(&file, temp_dir.path(), "test_api");
    let files = generator
        .generate(Language::Rust, side)
        .expect("Failed to generate");
    (temp_dir, files)
}

fn read_generated(temp_dir: &TempDir, filename: &str) -> String {
    let path = temp_dir.path().join("src").join(filename);
    std::fs::read_to_string(&path).unwrap_or_else(|_| {
        // Try without src/ prefix
        let alt_path = temp_dir.path().join(filename);
        std::fs::read_to_string(&alt_path).unwrap_or_default()
    })
}

// ============================================================================
// Model Generation Tests
// ============================================================================

#[test]
fn test_generate_simple_model() {
    let source = r#"
        model User {
            id: string;
            name: string;
            age: int32;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    assert!(models.contains("pub struct User"));
    assert!(models.contains("pub id: String"));
    assert!(models.contains("pub name: String"));
    assert!(models.contains("pub age: i32"));
}

#[test]
fn test_generate_model_with_optional_fields() {
    let source = r#"
        model Profile {
            username: string;
            bio?: string;
            avatar?: string;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    assert!(models.contains("pub username: String"));
    assert!(models.contains("pub bio: Option<String>"));
    assert!(models.contains("pub avatar: Option<String>"));
    assert!(models.contains("#[serde(skip_serializing_if = \"Option::is_none\")]"));
}

#[test]
fn test_generate_model_with_array_types() {
    let source = r#"
        model Container {
            items: string[];
            numbers: int32[];
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    assert!(models.contains("pub items: Vec<String>"));
    assert!(models.contains("pub numbers: Vec<i32>"));
}

#[test]
fn test_generate_model_with_record_type() {
    let source = r#"
        model Config {
            settings: Record<string>;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    assert!(models.contains("HashMap<String, String>"));
}

#[test]
fn test_generate_model_with_optional_record() {
    let source = r#"
        model Config {
            metadata?: Record<string>;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    assert!(models.contains("Option<HashMap<String, String>>"));
}

#[test]
fn test_generate_generic_model() {
    let source = r#"
        model PaginatedResponse<T> {
            items: T[];
            total: int32;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    assert!(models.contains("pub struct PaginatedResponse<T>"));
    assert!(models.contains("pub items: Vec<T>"));
}

#[test]
fn test_generate_model_with_datetime() {
    let source = r#"
        model Event {
            startTime: utcDateTime;
            date: plainDate;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    assert!(models.contains("DateTime<Utc>"));
    assert!(models.contains("chrono::NaiveDate"));
}

#[test]
fn test_generate_model_with_uuid() {
    let source = r#"
        @format("uuid")
        scalar uuid extends string;

        model Entity {
            id: uuid;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    assert!(models.contains("pub id: Uuid") || models.contains("pub id: String"));
}

#[test]
fn test_generate_model_camelcase_to_snake_case() {
    let source = r#"
        model User {
            firstName: string;
            lastName: string;
            createdAt: utcDateTime;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    assert!(models.contains("pub first_name: String"));
    assert!(models.contains("pub last_name: String"));
    assert!(models.contains("pub created_at: DateTime<Utc>"));
    assert!(models.contains("#[serde(rename_all = \"camelCase\")]"));
}

#[test]
fn test_generate_model_with_rust_keyword_field() {
    let source = r#"
        model Item {
            type: string;
            ref: string;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    // Rust keywords should be escaped with r#
    assert!(models.contains("r#type") || models.contains("pub type_:"));
}

// ============================================================================
// Enum Generation Tests
// ============================================================================

#[test]
fn test_generate_simple_enum() {
    let source = r#"
        enum Status {
            pending,
            active,
            completed,
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let enums = read_generated(&temp_dir, "enums.rs");

    assert!(enums.contains("pub enum Status"));
    assert!(enums.contains("Pending"));
    assert!(enums.contains("Active"));
    assert!(enums.contains("Completed"));
}

#[test]
fn test_generate_enum_with_explicit_values() {
    let source = r#"
        enum TaskStatus {
            pending,
            inProgress: "in_progress",
            completed,
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let enums = read_generated(&temp_dir, "enums.rs");

    assert!(enums.contains(r#"#[serde(rename = "in_progress")]"#));
    assert!(enums.contains("InProgress"));
}

#[test]
fn test_generate_inline_string_union_as_enum() {
    let source = r#"
        model Config {
            mode: "debug" | "release" | "test";
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let models = read_generated(&temp_dir, "models.rs");

    // Should generate an inline enum for string literal unions
    assert!(models.contains("ConfigMode") || models.contains("serde_json::Value"));
}

// ============================================================================
// Client Generation Tests
// ============================================================================

#[test]
fn test_generate_client_base() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get
            list(): string[];
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let client = read_generated(&temp_dir, "client.rs");

    assert!(client.contains("pub struct BaseClient"));
    assert!(client.contains("pub fn new(base_url: impl Into<String>)"));
    assert!(client.contains("pub fn with_token("));
}

#[test]
fn test_generate_service_client() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get
            list(): string[];
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let client = read_generated(&temp_dir, "client.rs");

    assert!(client.contains("pub struct UserServiceClient"));
    assert!(client.contains("pub async fn list(&self)"));
}

#[test]
fn test_generate_client_with_path_params() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get
            @route("/{id}")
            get(@path id: string): string;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let client = read_generated(&temp_dir, "client.rs");

    assert!(client.contains("pub async fn get(&self, id: &str)"));
}

#[test]
fn test_generate_client_with_body_params() {
    let source = r#"
        model CreateUserRequest {
            name: string;
        }

        @route("/users")
        interface UserService {
            @post
            create(@body body: CreateUserRequest): string;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let client = read_generated(&temp_dir, "client.rs");

    assert!(client.contains("body: &CreateUserRequest") || client.contains("body:"));
    assert!(client.contains("Method::POST"));
}

#[test]
fn test_generate_client_http_methods() {
    let source = r#"
        @route("/items")
        interface ItemService {
            @get list(): string[];
            @post create(): string;
            @put update(): string;
            @patch patch(): string;
            @delete remove(): string;
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Client);
    let client = read_generated(&temp_dir, "client.rs");

    assert!(client.contains("Method::GET"));
    assert!(client.contains("Method::POST"));
    assert!(client.contains("Method::PUT"));
    assert!(client.contains("Method::PATCH"));
    assert!(client.contains("Method::DELETE"));
}

// ============================================================================
// Server Generation Tests
// ============================================================================

#[test]
fn test_generate_server_trait() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get
            list(): string[];
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Server);
    let server = read_generated(&temp_dir, "server.rs");

    assert!(server.contains("pub trait UserServiceHandler"));
    assert!(server.contains("async fn list(&self)"));
}

#[test]
fn test_generate_server_skips_spread_params() {
    let source = r#"
        model PaginationParams {
            page?: int32;
        }

        @route("/items")
        interface ItemService {
            @get
            list(...PaginationParams): string[];
        }
    "#;

    let (temp_dir, _) = generate_rust(source, Side::Server);
    let server = read_generated(&temp_dir, "server.rs");

    // Should not have empty parameter names
    assert!(!server.contains(": PaginationParams)"));
    assert!(!server.contains(", : "));
}

// ============================================================================
// Side Selection Tests
// ============================================================================

#[test]
fn test_generate_client_only() {
    let source = r#"
        @route("/test")
        interface TestService {
            @get test(): string;
        }
    "#;

    let (temp_dir, files) = generate_rust(source, Side::Client);

    assert!(files.iter().any(|f| f.ends_with("client.rs")));
    assert!(!files.iter().any(|f| f.ends_with("server.rs")));
}

#[test]
fn test_generate_server_only() {
    let source = r#"
        @route("/test")
        interface TestService {
            @get test(): string;
        }
    "#;

    let (temp_dir, files) = generate_rust(source, Side::Server);

    assert!(!files.iter().any(|f| f.ends_with("client.rs")));
    assert!(files.iter().any(|f| f.ends_with("server.rs")));
}

#[test]
fn test_generate_both() {
    let source = r#"
        @route("/test")
        interface TestService {
            @get test(): string;
        }
    "#;

    let (temp_dir, files) = generate_rust(source, Side::Both);

    assert!(files.iter().any(|f| f.ends_with("client.rs")));
    assert!(files.iter().any(|f| f.ends_with("server.rs")));
}

// ============================================================================
// Cargo.toml Generation Tests
// ============================================================================

#[test]
fn test_generate_cargo_toml() {
    let source = "model Test { id: string; }";
    let (temp_dir, files) = generate_rust(source, Side::Both);

    assert!(files.iter().any(|f| f.ends_with("Cargo.toml")));

    let cargo_path = temp_dir.path().join("Cargo.toml");
    let cargo = std::fs::read_to_string(&cargo_path).unwrap();

    assert!(cargo.contains("[package]"));
    assert!(cargo.contains("serde"));
    assert!(cargo.contains("reqwest")); // client
    assert!(cargo.contains("axum")); // server
}

#[test]
fn test_generate_cargo_toml_client_only() {
    let source = "model Test { id: string; }";
    let (temp_dir, _) = generate_rust(source, Side::Client);

    let cargo_path = temp_dir.path().join("Cargo.toml");
    let cargo = std::fs::read_to_string(&cargo_path).unwrap();

    assert!(cargo.contains("reqwest"));
    assert!(!cargo.contains("axum"));
}

// ============================================================================
// lib.rs Generation Tests
// ============================================================================

#[test]
fn test_generate_lib_rs() {
    let source = "model Test { id: string; }";
    let (temp_dir, files) = generate_rust(source, Side::Both);

    assert!(files.iter().any(|f| f.ends_with("lib.rs")));

    let lib = read_generated(&temp_dir, "lib.rs");
    assert!(lib.contains("pub mod models;"));
    assert!(lib.contains("pub mod enums;"));
    assert!(lib.contains("pub mod client;"));
    assert!(lib.contains("pub mod server;"));
}
