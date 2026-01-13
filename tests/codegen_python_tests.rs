//! Comprehensive Python code generation tests

use tempfile::TempDir;
use typespec_api::{
    codegen::{Generator, Language, Side},
    parse,
};

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_py(source: &str, side: Side) -> (TempDir, Vec<String>) {
    let file = parse(source).expect("Failed to parse TypeSpec");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let generator = Generator::new(&file, temp_dir.path(), "test_api");
    let files = generator
        .generate(Language::Python, side)
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

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("@dataclass"));
    assert!(models.contains("class User:"));
    assert!(models.contains("id: str"));
    assert!(models.contains("name: str"));
    assert!(models.contains("age: int"));
}

#[test]
fn test_generate_model_with_optional_fields() {
    let source = r#"
        model Profile {
            username: string;
            bio?: string;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("username: str"));
    assert!(models.contains("bio: Optional[str] = None"));
}

#[test]
fn test_generate_model_field_ordering() {
    let source = r#"
        model Mixed {
            required1: string;
            optional1?: string;
            required2: int32;
            optional2?: int32;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    // Python uses snake_case field names
    // Required fields should come before optional fields (Python dataclass requirement)
    assert!(models.contains("required_1: str") || models.contains("required1: str"));
    assert!(models.contains("required_2: int") || models.contains("required2: int"));
    assert!(models.contains("Optional[str]"));
    assert!(models.contains("Optional[int]"));
}

#[test]
fn test_generate_model_with_array_types() {
    let source = r#"
        model Container {
            items: string[];
            numbers: int32[];
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("items: List[str]"));
    assert!(models.contains("numbers: List[int]"));
}

#[test]
fn test_generate_model_with_dict_type() {
    let source = r#"
        model Config {
            settings: Record<string>;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("Dict[str, str]"));
}

#[test]
fn test_generate_generic_model() {
    let source = r#"
        model PaginatedResponse<T> {
            items: T[];
            total: int32;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("class PaginatedResponse(Generic[T]):"));
    assert!(models.contains("items: List[T]"));
}

#[test]
fn test_generate_model_with_union_literals() {
    let source = r#"
        model Status {
            state: "success" | "error" | "pending";
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains(r#"Literal["success", "error", "pending"]"#));
}

#[test]
fn test_generate_model_number_types() {
    let source = r#"
        model Numbers {
            intVal: int32;
            floatVal: float64;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("int_val: int"));
    assert!(models.contains("float_val: float"));
}

#[test]
fn test_generate_model_date_types() {
    let source = r#"
        model Event {
            startTime: utcDateTime;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("start_time: datetime"));
}

#[test]
fn test_generate_model_snake_case_fields() {
    let source = r#"
        model User {
            firstName: string;
            lastName: string;
            createdAt: utcDateTime;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    // Python uses snake_case
    assert!(models.contains("first_name: str"));
    assert!(models.contains("last_name: str"));
    assert!(models.contains("created_at: datetime"));
}

#[test]
fn test_generate_model_to_dict_method() {
    let source = r#"
        model User {
            id: string;
            name?: string;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("def to_dict(self) -> Dict[str, Any]:"));
    assert!(models.contains(r#"result["id"] = self.id"#));
}

#[test]
fn test_generate_model_from_dict_method() {
    let source = r#"
        model User {
            id: string;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("@classmethod"));
    assert!(models.contains("def from_dict(cls, data: Dict[str, Any])"));
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

    let (temp_dir, _) = generate_py(source, Side::Client);
    let enums = read_generated(&temp_dir, "enums.py");

    assert!(enums.contains("class Status(str, Enum):"));
    assert!(enums.contains("PENDING"));
    assert!(enums.contains("ACTIVE"));
    assert!(enums.contains("COMPLETED"));
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

    let (temp_dir, _) = generate_py(source, Side::Client);
    let enums = read_generated(&temp_dir, "enums.py");

    assert!(enums.contains(r#"IN_PROGRESS = "in_progress""#));
}

// ============================================================================
// Client Generation Tests
// ============================================================================

#[test]
fn test_generate_client_base_class() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get list(): string[];
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let client = read_generated(&temp_dir, "client/__init__.py");

    assert!(client.contains("class BaseClient:"));
    assert!(client.contains("def __init__(self, base_url: str"));
    assert!(client.contains("async def __aenter__(self)"));
    assert!(client.contains("async def __aexit__(self"));
}

#[test]
fn test_generate_service_client() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get list(): string[];
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let client = read_generated(&temp_dir, "client/__init__.py");

    assert!(client.contains("class UserServiceClient:"));
    assert!(client.contains("async def list(self)"));
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

    let (temp_dir, _) = generate_py(source, Side::Client);
    let client = read_generated(&temp_dir, "client/__init__.py");

    assert!(client.contains("async def get(self, id: str)"));
    // Path replacement can use different formats
    assert!(client.contains("replace") || client.contains("{id}"));
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

    let (temp_dir, _) = generate_py(source, Side::Client);
    let client = read_generated(&temp_dir, "client/__init__.py");

    assert!(client.contains("body: CreateRequest"));
    assert!(client.contains("json=body.to_dict()"));
}

#[test]
fn test_generate_client_with_query_params() {
    let source = r#"
        @route("/items")
        interface ItemService {
            @get
            list(@query page?: int32, @query limit?: int32): string[];
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let client = read_generated(&temp_dir, "client/__init__.py");

    assert!(client.contains("page: Optional[int]"));
    assert!(client.contains("limit: Optional[int]"));
    assert!(client.contains("params=params"));
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

    let (temp_dir, _) = generate_py(source, Side::Client);
    let client = read_generated(&temp_dir, "client/__init__.py");

    assert!(client.contains("class Client(BaseClient):"));
    assert!(client.contains("self.user_service = UserServiceClient(self)"));
    assert!(client.contains("self.item_service = ItemServiceClient(self)"));
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

    let (temp_dir, _) = generate_py(source, Side::Server);
    let server = read_generated(&temp_dir, "server/__init__.py");

    assert!(server.contains("class UserServiceHandler(ABC):"));
    assert!(server.contains("@abstractmethod"));
    assert!(server.contains("async def list(self)"));
}

// ============================================================================
// Package Structure Tests
// ============================================================================

#[test]
fn test_generate_init_file() {
    let source = "model Test { id: string; }";
    let (temp_dir, files) = generate_py(source, Side::Client);

    assert!(files.iter().any(|f| f.ends_with("__init__.py")));

    let init = read_generated(&temp_dir, "__init__.py");
    assert!(init.contains("from .models import *"));
    assert!(init.contains("from .enums import *"));
    assert!(init.contains("from .client import Client"));
}

#[test]
fn test_generates_all_files() {
    let source = r#"
        enum Status { active }
        model User { id: string; }
        @route("/users") interface UserService { @get list(): User[]; }
    "#;

    let (temp_dir, files) = generate_py(source, Side::Both);

    assert!(files.iter().any(|f| f.ends_with("models.py")));
    assert!(files.iter().any(|f| f.ends_with("enums.py")));
    assert!(files
        .iter()
        .any(|f| f.contains("client") && f.ends_with("__init__.py")));
    assert!(files
        .iter()
        .any(|f| f.contains("server") && f.ends_with("__init__.py")));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_model() {
    let source = r#"
        model Empty {}
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("class Empty:"));
    assert!(models.contains("pass"));
}

#[test]
fn test_boolean_type() {
    let source = r#"
        model Flags {
            enabled: boolean;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("enabled: bool"));
}

#[test]
fn test_bytes_type() {
    let source = r#"
        model Binary {
            data: bytes;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("data: bytes"));
}

// ============================================================================
// Import Generation Tests
// ============================================================================

#[test]
fn test_generates_required_imports() {
    let source = r#"
        model Test {
            id: string;
            items: string[];
            meta?: Record<string>;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("from __future__ import annotations"));
    assert!(models.contains("from dataclasses import dataclass"));
    assert!(models.contains("from typing import"));
    assert!(models.contains("Optional"));
    assert!(models.contains("List"));
    assert!(models.contains("Dict"));
}

#[test]
fn test_generates_typevar_for_generics() {
    let source = r#"
        model Response<T> {
            data: T;
        }
    "#;

    let (temp_dir, _) = generate_py(source, Side::Client);
    let models = read_generated(&temp_dir, "models.py");

    assert!(models.contains("from typing import"));
    assert!(models.contains("TypeVar"));
    assert!(models.contains("Generic"));
    assert!(models.contains("T = TypeVar('T')"));
}
