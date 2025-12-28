//! Comprehensive parser tests for TypeSpec AST generation

use typespec_api::{parse, ast::*};

// ============================================================================
// Model Parsing Tests
// ============================================================================

#[test]
fn test_parse_simple_model() {
    let source = r#"
        model User {
            id: string;
            name: string;
        }
    "#;

    let file = parse(source).unwrap();
    assert_eq!(file.models().count(), 1);

    let model = file.models().next().unwrap();
    assert_eq!(model.name, "User");
    assert_eq!(model.properties.len(), 2);
    assert_eq!(model.properties[0].name, "id");
    assert_eq!(model.properties[1].name, "name");
}

#[test]
fn test_parse_model_with_optional_fields() {
    let source = r#"
        model Profile {
            username: string;
            bio?: string;
            avatar?: string;
        }
    "#;

    let file = parse(source).unwrap();
    let model = file.models().next().unwrap();

    assert!(!model.properties[0].optional); // username
    assert!(model.properties[1].optional);  // bio?
    assert!(model.properties[2].optional);  // avatar?
}

#[test]
fn test_parse_model_with_decorators() {
    let source = r#"
        @doc("A user in the system")
        model User {
            @key
            id: string;
            @minLength(1)
            name: string;
        }
    "#;

    let file = parse(source).unwrap();
    let model = file.models().next().unwrap();

    assert!(model.decorators.iter().any(|d| d.name == "doc"));
    assert!(model.properties[0].decorators.iter().any(|d| d.name == "key"));
    assert!(model.properties[1].decorators.iter().any(|d| d.name == "minLength"));
}

#[test]
fn test_parse_model_with_type_params() {
    let source = r#"
        model PaginatedResponse<T> {
            items: T[];
            total: int32;
            hasMore: boolean;
        }
    "#;

    let file = parse(source).unwrap();
    let model = file.models().next().unwrap();

    assert_eq!(model.name, "PaginatedResponse");
    assert_eq!(model.type_params, vec!["T".to_string()]);
    assert_eq!(model.properties.len(), 3);
}

#[test]
fn test_parse_model_with_multiple_type_params() {
    let source = r#"
        model KeyValue<K, V> {
            key: K;
            value: V;
        }
    "#;

    let file = parse(source).unwrap();
    let model = file.models().next().unwrap();

    assert_eq!(model.type_params, vec!["K".to_string(), "V".to_string()]);
}

#[test]
fn test_parse_model_with_spread() {
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

    let file = parse(source).unwrap();
    let user = file.models().find(|m| m.name == "User").unwrap();

    assert_eq!(user.properties.len(), 2);  // id, name
    assert_eq!(user.spread_refs.len(), 1); // ...Timestamps
}

#[test]
fn test_parse_model_with_extends() {
    let source = r#"
        model AdminUser extends User {
            permissions: string[];
        }
    "#;

    let file = parse(source).unwrap();
    let model = file.models().next().unwrap();

    assert!(model.extends.is_some());
    assert!(matches!(&model.extends, Some(TypeRef::Named(n)) if n == "User"));
}

// ============================================================================
// Enum Parsing Tests
// ============================================================================

#[test]
fn test_parse_simple_enum() {
    let source = r#"
        enum Status {
            pending,
            active,
            completed,
        }
    "#;

    let file = parse(source).unwrap();
    assert_eq!(file.enums().count(), 1);

    let enum_def = file.enums().next().unwrap();
    assert_eq!(enum_def.name, "Status");
    assert_eq!(enum_def.members.len(), 3);
    assert_eq!(enum_def.members[0].name, "pending");
    assert_eq!(enum_def.members[1].name, "active");
    assert_eq!(enum_def.members[2].name, "completed");
}

#[test]
fn test_parse_enum_with_explicit_values() {
    let source = r#"
        enum TaskStatus {
            pending,
            inProgress: "in_progress",
            completed,
        }
    "#;

    let file = parse(source).unwrap();
    let enum_def = file.enums().next().unwrap();

    assert!(enum_def.members[0].value.is_none()); // pending (implicit)
    assert!(matches!(
        &enum_def.members[1].value,
        Some(Value::String(s)) if s == "in_progress"
    ));
    assert!(enum_def.members[2].value.is_none()); // completed (implicit)
}

// ============================================================================
// Interface Parsing Tests
// ============================================================================

#[test]
fn test_parse_simple_interface() {
    let source = r#"
        @route("/users")
        interface UserService {
            @get
            list(): User[];

            @post
            create(@body body: CreateUserRequest): User;
        }
    "#;

    let file = parse(source).unwrap();
    assert_eq!(file.interfaces().count(), 1);

    let iface = file.interfaces().next().unwrap();
    assert_eq!(iface.name, "UserService");
    assert_eq!(iface.operations.len(), 2);

    // Check route decorator
    let route = iface.decorators.iter().find(|d| d.name == "route").unwrap();
    assert_eq!(route.get_string_arg(0), Some("/users"));
}

#[test]
fn test_parse_interface_operations() {
    let source = r#"
        interface TaskService {
            @get
            @route("/{id}")
            get(@path id: string): Task;

            @patch
            @route("/{id}")
            update(@path id: string, @body body: UpdateTaskRequest): Task;

            @delete
            @route("/{id}")
            delete(@path id: string): void;
        }
    "#;

    let file = parse(source).unwrap();
    let iface = file.interfaces().next().unwrap();

    // Check GET operation
    let get_op = &iface.operations[0];
    assert_eq!(get_op.name, "get");
    assert!(get_op.decorators.iter().any(|d| d.name == "get"));
    assert_eq!(get_op.params.len(), 1);
    assert!(get_op.params[0].decorators.iter().any(|d| d.name == "path"));

    // Check PATCH operation
    let update_op = &iface.operations[1];
    assert_eq!(update_op.params.len(), 2);

    // Check DELETE operation
    let delete_op = &iface.operations[2];
    assert!(delete_op.decorators.iter().any(|d| d.name == "delete"));
}

#[test]
fn test_parse_interface_with_spread_params() {
    let source = r#"
        model PaginationParams {
            page?: int32;
            limit?: int32;
        }

        interface ItemService {
            @get
            list(...PaginationParams): Item[];
        }
    "#;

    let file = parse(source).unwrap();
    let iface = file.interfaces().next().unwrap();
    let list_op = &iface.operations[0];

    assert_eq!(list_op.params.len(), 1);
    assert!(list_op.params[0].spread);
}

// ============================================================================
// Type Reference Tests
// ============================================================================

#[test]
fn test_parse_builtin_types() {
    let source = r#"
        model TypeDemo {
            str: string;
            num32: int32;
            num64: int64;
            bool_val: boolean;
            date: utcDateTime;
        }
    "#;

    let file = parse(source).unwrap();
    let model = file.models().next().unwrap();

    assert!(matches!(&model.properties[0].type_ref, TypeRef::Builtin(s) if s == "string"));
    assert!(matches!(&model.properties[1].type_ref, TypeRef::Builtin(s) if s == "int32"));
    assert!(matches!(&model.properties[2].type_ref, TypeRef::Builtin(s) if s == "int64"));
    assert!(matches!(&model.properties[3].type_ref, TypeRef::Builtin(s) if s == "boolean"));
    assert!(matches!(&model.properties[4].type_ref, TypeRef::Builtin(s) if s == "utcDateTime"));
}

#[test]
fn test_parse_array_types() {
    let source = r#"
        model Container {
            items: string[];
            nested: int32[][];
        }
    "#;

    let file = parse(source).unwrap();
    let model = file.models().next().unwrap();

    assert!(matches!(&model.properties[0].type_ref, TypeRef::Array(inner) if matches!(&**inner, TypeRef::Builtin(s) if s == "string")));
}

#[test]
fn test_parse_generic_types() {
    let source = r#"
        model Response {
            data: PaginatedResponse<User>;
            metadata: Record<string>;
        }
    "#;

    let file = parse(source).unwrap();
    let model = file.models().next().unwrap();

    if let TypeRef::Generic { base, args } = &model.properties[0].type_ref {
        assert!(matches!(&**base, TypeRef::Named(n) if n == "PaginatedResponse"));
        assert_eq!(args.len(), 1);
    } else {
        panic!("Expected generic type");
    }
}

#[test]
fn test_parse_union_types() {
    let source = r#"
        model Status {
            state: "success" | "error" | "pending";
        }
    "#;

    let file = parse(source).unwrap();
    let model = file.models().next().unwrap();

    if let TypeRef::Union(variants) = &model.properties[0].type_ref {
        assert_eq!(variants.len(), 3);
        assert!(matches!(&variants[0], TypeRef::StringLiteral(s) if s == "success"));
        assert!(matches!(&variants[1], TypeRef::StringLiteral(s) if s == "error"));
        assert!(matches!(&variants[2], TypeRef::StringLiteral(s) if s == "pending"));
    } else {
        panic!("Expected union type");
    }
}

// ============================================================================
// Scalar Parsing Tests
// ============================================================================

#[test]
fn test_parse_scalar() {
    let source = r#"
        @format("uuid")
        scalar uuid extends string;
    "#;

    let file = parse(source).unwrap();
    let scalar = file.scalars().next().unwrap();

    assert_eq!(scalar.name, "uuid");
    assert_eq!(scalar.extends, Some("string".to_string()));
    assert!(scalar.decorators.iter().any(|d| d.name == "format"));
}

// ============================================================================
// Import and Using Tests
// ============================================================================

#[test]
fn test_parse_imports() {
    let source = r#"
        import "@typespec/http";
        import "./common.tsp";
    "#;

    let file = parse(source).unwrap();
    assert_eq!(file.imports.len(), 2);
    assert_eq!(file.imports[0].path, "@typespec/http");
    assert_eq!(file.imports[1].path, "./common.tsp");
}

#[test]
fn test_parse_using() {
    let source = r#"
        using TypeSpec.Http;
        using MyNamespace.Models;
    "#;

    let file = parse(source).unwrap();
    assert_eq!(file.usings.len(), 2);
    assert_eq!(file.usings[0].namespace, "TypeSpec.Http");
    assert_eq!(file.usings[1].namespace, "MyNamespace.Models");
}

// ============================================================================
// Namespace Tests
// ============================================================================

#[test]
fn test_parse_namespace_declaration() {
    let source = r#"
        namespace MyApi;

        model User {
            id: string;
        }
    "#;

    let file = parse(source).unwrap();
    assert_eq!(file.namespace, Some("MyApi".to_string()));
    assert_eq!(file.models().count(), 1);
}

#[test]
fn test_parse_qualified_namespace() {
    let source = r#"
        namespace MyCompany.MyApi.V1;
    "#;

    let file = parse(source).unwrap();
    assert_eq!(file.namespace, Some("MyCompany.MyApi.V1".to_string()));
}

// ============================================================================
// Decorator Arguments Tests
// ============================================================================

#[test]
fn test_parse_decorator_string_arg() {
    let source = r#"
        @route("/users/{id}")
        interface Test {}
    "#;

    let file = parse(source).unwrap();
    let iface = file.interfaces().next().unwrap();
    let route = iface.decorators.iter().find(|d| d.name == "route").unwrap();

    assert_eq!(route.get_string_arg(0), Some("/users/{id}"));
}

#[test]
fn test_parse_decorator_number_arg() {
    let source = r#"
        model Test {
            @minLength(1)
            @maxLength(100)
            name: string;
        }
    "#;

    let file = parse(source).unwrap();
    let model = file.models().next().unwrap();

    assert!(model.properties[0].decorators.iter().any(|d| d.name == "minLength"));
    assert!(model.properties[0].decorators.iter().any(|d| d.name == "maxLength"));
}

#[test]
fn test_parse_decorator_named_args() {
    let source = r#"
        @service({ title: "My API", version: "1.0" })
        namespace MyApi;
    "#;

    let file = parse(source).unwrap();
    // Service decorator is on the namespace block in nested form
    // This test verifies named argument parsing doesn't crash
}

// ============================================================================
// Alias Tests
// ============================================================================

#[test]
fn test_parse_alias() {
    let source = r#"
        alias StringArray = string[];
    "#;

    let file = parse(source).unwrap();
    let decls: Vec<_> = file.declarations.iter().collect();

    assert!(matches!(
        &decls[0],
        Declaration::Alias(a) if a.name == "StringArray"
    ));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_parse_invalid_syntax_returns_error() {
    let source = "model { }"; // missing name
    let result = parse(source);
    assert!(result.is_err());
}

#[test]
fn test_parse_unclosed_brace_returns_error() {
    let source = "model User {";
    let result = parse(source);
    assert!(result.is_err());
}

// ============================================================================
// Complex Integration Tests
// ============================================================================

#[test]
fn test_parse_full_api_definition() {
    let source = r#"
        import "@typespec/http";
        using TypeSpec.Http;

        namespace MyApi;

        @format("uuid")
        scalar uuid extends string;

        model Timestamps {
            createdAt: utcDateTime;
            updatedAt: utcDateTime;
        }

        model User {
            id: uuid;
            name: string;
            email?: string;
            ...Timestamps;
        }

        enum UserRole {
            admin,
            user,
            guest,
        }

        @route("/users")
        interface UserService {
            @get
            list(): User[];

            @get
            @route("/{id}")
            get(@path id: uuid): User;

            @post
            create(@body body: User): User;
        }
    "#;

    let file = parse(source).unwrap();

    assert_eq!(file.imports.len(), 1);
    assert_eq!(file.usings.len(), 1);
    assert_eq!(file.namespace, Some("MyApi".to_string()));
    assert_eq!(file.scalars().count(), 1);
    assert_eq!(file.models().count(), 2);
    assert_eq!(file.enums().count(), 1);
    assert_eq!(file.interfaces().count(), 1);
}
