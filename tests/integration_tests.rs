//! Integration tests that verify generated code compiles and is correct

use typespec_api::{parse, codegen::{Generator, Language, Side}};
use tempfile::TempDir;
use std::process::Command;

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_and_check_rust(source: &str) -> Result<(), String> {
    let file = parse(source).map_err(|e| format!("Parse error: {}", e))?;
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let generator = Generator::new(&file, temp_dir.path(), "test_api");
    generator.generate(Language::Rust, Side::Both)
        .map_err(|e| format!("Generation error: {}", e))?;

    // Run cargo check on the generated code
    let output = Command::new("cargo")
        .arg("check")
        .current_dir(temp_dir.path())
        .output()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Cargo check failed:\n{}", stderr));
    }

    Ok(())
}

// ============================================================================
// Rust Compilation Tests
// ============================================================================

#[test]
fn test_compile_simple_model() {
    let source = r#"
        model User {
            id: string;
            name: string;
            age: int32;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile simple model: {}", e);
    }
}

#[test]
fn test_compile_model_with_optional_fields() {
    let source = r#"
        model Profile {
            username: string;
            bio?: string;
            avatar?: string;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile model with optional fields: {}", e);
    }
}

#[test]
fn test_compile_model_with_record() {
    let source = r#"
        model Config {
            required_settings: Record<string>;
            optional_settings?: Record<string>;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile model with Record: {}", e);
    }
}

#[test]
fn test_compile_model_with_arrays() {
    let source = r#"
        model Container {
            items: string[];
            numbers: int32[];
            optional_items?: string[];
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile model with arrays: {}", e);
    }
}

#[test]
fn test_compile_model_with_datetime() {
    let source = r#"
        model Event {
            startTime: utcDateTime;
            endTime?: utcDateTime;
            date: plainDate;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile model with datetime: {}", e);
    }
}

#[test]
fn test_compile_generic_model() {
    let source = r#"
        model PaginatedResponse<T> {
            items: T[];
            total: int32;
            hasMore: boolean;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile generic model: {}", e);
    }
}

#[test]
fn test_compile_enum() {
    let source = r#"
        enum Status {
            pending,
            active,
            completed,
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile enum: {}", e);
    }
}

#[test]
fn test_compile_enum_with_explicit_values() {
    let source = r#"
        enum TaskStatus {
            pending,
            inProgress: "in_progress",
            completed,
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile enum with explicit values: {}", e);
    }
}

#[test]
fn test_compile_model_with_enum_reference() {
    let source = r#"
        enum Status {
            pending,
            active,
        }

        model Task {
            id: string;
            status: Status;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile model with enum reference: {}", e);
    }
}

#[test]
fn test_compile_interface_client() {
    let source = r#"
        model User {
            id: string;
            name: string;
        }

        @route("/users")
        interface UserService {
            @get
            list(): User[];

            @get
            @route("/{id}")
            get(@path id: string): User;

            @post
            create(@body body: User): User;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile interface client: {}", e);
    }
}

#[test]
fn test_compile_model_with_all_builtin_types() {
    let source = r#"
        model AllTypes {
            str: string;
            i8: int8;
            i16: int16;
            i32: int32;
            i64: int64;
            u8: uint8;
            u16: uint16;
            u32: uint32;
            u64: uint64;
            f32: float32;
            f64: float64;
            flag: boolean;
            data: bytes;
            dateTime: utcDateTime;
            date: plainDate;
            time: plainTime;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile model with all builtin types: {}", e);
    }
}

#[test]
fn test_compile_model_with_spread() {
    let source = r#"
        model Timestamps {
            createdAt: utcDateTime;
            updatedAt: utcDateTime;
        }

        model User {
            id: string;
            name: string;
            ...Timestamps;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile model with spread: {}", e);
    }
}

#[test]
fn test_compile_model_with_string_union() {
    let source = r#"
        model Config {
            mode: "debug" | "release" | "test";
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile model with string union: {}", e);
    }
}

#[test]
fn test_compile_scalar_types() {
    let source = r#"
        @format("uuid")
        scalar uuid extends string;

        @format("email")
        scalar email extends string;

        model User {
            id: uuid;
            contactEmail: email;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile scalar types: {}", e);
    }
}

#[test]
fn test_compile_complex_api() {
    let source = r#"
        @format("uuid")
        scalar uuid extends string;

        model Timestamps {
            createdAt: utcDateTime;
            updatedAt: utcDateTime;
        }

        enum TaskStatus {
            pending,
            inProgress: "in_progress",
            completed,
        }

        enum TaskPriority {
            low,
            medium,
            high,
        }

        model Task {
            id: uuid;
            title: string;
            description?: string;
            status: TaskStatus;
            priority: TaskPriority;
            tags: string[];
            metadata?: Record<string>;
            ...Timestamps;
        }

        model CreateTaskRequest {
            title: string;
            description?: string;
            priority?: TaskPriority;
            tags?: string[];
        }

        model PaginatedResponse<T> {
            items: T[];
            total: int32;
            page: int32;
            limit: int32;
            hasMore: boolean;
        }

        @route("/tasks")
        interface TaskService {
            @get
            list(): PaginatedResponse<Task>;

            @get
            @route("/{id}")
            get(@path id: string): Task;

            @post
            create(@body body: CreateTaskRequest): Task;

            @patch
            @route("/{id}")
            update(@path id: string, @body body: CreateTaskRequest): Task;

            @delete
            @route("/{id}")
            delete(@path id: string): void;

            @post
            @route("/{id}/complete")
            complete(@path id: string): Task;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile complex API: {}", e);
    }
}

// ============================================================================
// Edge Case Compilation Tests
// ============================================================================

#[test]
fn test_compile_rust_keyword_fields() {
    let source = r#"
        model Item {
            type: string;
            ref: string;
            loop: int32;
            match: boolean;
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile model with Rust keyword fields: {}", e);
    }
}

#[test]
fn test_compile_empty_model() {
    let source = r#"
        model Empty {}
    "#;

    // Empty models should still compile (will have no fields)
    let file = parse(source).unwrap();
    let temp_dir = TempDir::new().unwrap();
    let generator = Generator::new(&file, temp_dir.path(), "test_api");
    let result = generator.generate(Language::Rust, Side::Client);
    assert!(result.is_ok());
}

#[test]
fn test_compile_deeply_nested_generics() {
    let source = r#"
        model Response<T> {
            data: T;
        }

        model Container {
            items: Response<string>[];
        }
    "#;

    if let Err(e) = generate_and_check_rust(source) {
        panic!("Failed to compile deeply nested generics: {}", e);
    }
}
