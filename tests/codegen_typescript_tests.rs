//! Comprehensive TypeScript code generation tests

use tempfile::TempDir;
use typespec_api::{
    codegen::{Generator, Language, Side},
    parse,
};

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_ts(source: &str, side: Side) -> (TempDir, Vec<String>) {
    let file = parse(source).expect("Failed to parse TypeSpec");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let generator = Generator::new(&file, temp_dir.path(), "test_api");
    let files = generator
        .generate(Language::TypeScript, side)
        .expect("Failed to generate");
    (temp_dir, files)
}

fn read_generated(temp_dir: &TempDir, filename: &str) -> String {
    let path = temp_dir.path().join(filename);
    std::fs::read_to_string(&path).unwrap_or_default()
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

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    assert!(models.contains("export interface User"));
    assert!(models.contains("id: string"));
    assert!(models.contains("name: string"));
    assert!(models.contains("age: number"));
}

#[test]
fn test_generate_model_with_optional_fields() {
    let source = r#"
        model Profile {
            username: string;
            bio?: string;
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    assert!(models.contains("username: string"));
    assert!(models.contains("bio?: string"));
}

#[test]
fn test_generate_model_with_array_types() {
    let source = r#"
        model Container {
            items: string[];
            numbers: int32[];
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    assert!(models.contains("items: string[]"));
    assert!(models.contains("numbers: number[]"));
}

#[test]
fn test_generate_model_with_record_type() {
    let source = r#"
        model Config {
            settings: Record<string>;
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    assert!(models.contains("Record<string, string>"));
}

#[test]
fn test_generate_generic_model() {
    let source = r#"
        model PaginatedResponse<T> {
            items: T[];
            total: int32;
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    assert!(models.contains("export interface PaginatedResponse<T>"));
    assert!(models.contains("items: T[]"));
}

#[test]
fn test_generate_model_with_multiple_type_params() {
    let source = r#"
        model KeyValue<K, V> {
            key: K;
            value: V;
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    assert!(models.contains("export interface KeyValue<K, V>"));
}

#[test]
fn test_generate_model_with_union_types() {
    let source = r#"
        model Status {
            state: "success" | "error" | "pending";
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    assert!(models.contains("'success' | 'error' | 'pending'"));
}

#[test]
fn test_generate_model_number_types() {
    let source = r#"
        model Numbers {
            int8Val: int8;
            int16Val: int16;
            int32Val: int32;
            int64Val: int64;
            float32Val: float32;
            float64Val: float64;
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    // All numeric types map to 'number' in TypeScript
    assert!(models.contains("int8Val: number"));
    assert!(models.contains("int32Val: number"));
    assert!(models.contains("float64Val: number"));
}

#[test]
fn test_generate_model_date_types() {
    let source = r#"
        model Event {
            startTime: utcDateTime;
            date: plainDate;
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    // DateTime types are strings in TypeScript (ISO format)
    assert!(models.contains("startTime: string"));
    assert!(models.contains("date: string"));
}

#[test]
fn test_generate_model_preserves_field_names() {
    let source = r#"
        model User {
            firstName: string;
            lastName: string;
            createdAt: utcDateTime;
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    // TypeScript keeps camelCase
    assert!(models.contains("firstName: string"));
    assert!(models.contains("lastName: string"));
    assert!(models.contains("createdAt: string"));
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

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let enums = read_generated(&temp_dir, "enums.ts");

    assert!(enums.contains("export enum Status"));
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

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let enums = read_generated(&temp_dir, "enums.ts");

    assert!(enums.contains(r#"InProgress = "in_progress""#));
}

// ============================================================================
// Client Generation Tests
// ============================================================================

#[test]
fn test_generate_client_classes() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get
            list(): string[];
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let client = read_generated(&temp_dir, "client.ts");

    assert!(client.contains("export class BaseClient"));
    assert!(client.contains("export class UserServiceClient"));
    assert!(client.contains("export class Client"));
}

#[test]
fn test_generate_client_methods() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get list(): string[];
            @post create(): string;
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let client = read_generated(&temp_dir, "client.ts");

    assert!(client.contains("async list("));
    assert!(client.contains("async create("));
    assert!(client.contains("'GET'"));
    assert!(client.contains("'POST'"));
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

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let client = read_generated(&temp_dir, "client.ts");

    assert!(client.contains("id: string"));
    assert!(client.contains("${id}") || client.contains("{id}"));
}

#[test]
fn test_generate_client_with_body() {
    let source = r#"
        model CreateRequest {
            name: string;
        }

        @route("/items")
        interface ItemService {
            @post
            create(@body body: CreateRequest): string;
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let client = read_generated(&temp_dir, "client.ts");

    assert!(client.contains("body: CreateRequest"));
    assert!(client.contains("body:"));
}

#[test]
fn test_generate_client_with_query_params() {
    let source = r#"
        @route("/items")
        interface ItemService {
            @get
            list(@query page: int32, @query limit: int32): string[];
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let client = read_generated(&temp_dir, "client.ts");

    assert!(client.contains("page: number"));
    assert!(client.contains("limit: number"));
    assert!(client.contains("query:"));
}

#[test]
fn test_generate_main_client() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get list(): string[];
        }

        @route("/items")
        interface ItemService {
            @get list(): string[];
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let client = read_generated(&temp_dir, "client.ts");

    // Main Client class should have both services
    assert!(client.contains("readonly userService: UserServiceClient"));
    assert!(client.contains("readonly itemService: ItemServiceClient"));
}

// ============================================================================
// Server Generation Tests
// ============================================================================

#[test]
fn test_generate_server_abstract_class() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get list(): string[];
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Server);
    let server = read_generated(&temp_dir, "server.ts");

    assert!(server.contains("export abstract class UserServiceHandler"));
    assert!(server.contains("abstract list("));
}

// ============================================================================
// Index Generation Tests
// ============================================================================

#[test]
fn test_generate_index_exports() {
    let source = "model Test { id: string; }";
    let (temp_dir, files) = generate_ts(source, Side::Both);

    assert!(files.iter().any(|f| f.ends_with("index.ts")));

    let index = read_generated(&temp_dir, "index.ts");
    assert!(index.contains("export * from './models'"));
    assert!(index.contains("export * from './enums'"));
    assert!(index.contains("export * from './client'"));
    assert!(index.contains("export * from './server'"));
}

#[test]
fn test_generate_index_client_only() {
    let source = "model Test { id: string; }";
    let (temp_dir, _) = generate_ts(source, Side::Client);

    let index = read_generated(&temp_dir, "index.ts");
    assert!(index.contains("export * from './client'"));
    assert!(!index.contains("export * from './server'"));
}

// ============================================================================
// File Structure Tests
// ============================================================================

#[test]
fn test_generates_all_files() {
    let source = r#"
        enum Status { active }
        model User { id: string; }
        @route("/users") interface UserService { @get list(): User[]; }
    "#;

    let (temp_dir, files) = generate_ts(source, Side::Both);

    assert!(files.iter().any(|f| f.ends_with("models.ts")));
    assert!(files.iter().any(|f| f.ends_with("enums.ts")));
    assert!(files.iter().any(|f| f.ends_with("client.ts")));
    assert!(files.iter().any(|f| f.ends_with("server.ts")));
    assert!(files.iter().any(|f| f.ends_with("index.ts")));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_model() {
    let source = r#"
        model Empty {}
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    assert!(models.contains("export interface Empty"));
}

#[test]
fn test_bytes_type() {
    let source = r#"
        model Binary {
            data: bytes;
        }
    "#;

    let (temp_dir, _) = generate_ts(source, Side::Client);
    let models = read_generated(&temp_dir, "models.ts");

    assert!(models.contains("Uint8Array"));
}
