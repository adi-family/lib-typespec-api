//! TypeSpec Parser and Code Generator
//!
//! Pure Rust implementation that:
//! - Parses TypeSpec (.tsp) files directly
//! - Generates Python, TypeScript, and Rust code

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod codegen;

pub use ast::*;
pub use parser::parse;
pub use codegen::{Generator, Language, Side};

#[cfg(test)]
mod tests {
    use super::*;
    use codegen::{build_model_map, resolve_properties};

    #[test]
    fn test_spread_operator_resolution() {
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
        let models = build_model_map(&file);

        // Find the User model
        let user = file.models().find(|m| m.name == "User").unwrap();
        let properties = resolve_properties(user, &models);

        // Should have 4 properties: id, name, createdAt, updatedAt
        assert_eq!(properties.len(), 4);

        let names: Vec<_> = properties.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"id"));
        assert!(names.contains(&"name"));
        assert!(names.contains(&"createdAt"));
        assert!(names.contains(&"updatedAt"));
    }

    #[test]
    fn test_nested_spread_resolution() {
        let source = r#"
            model Base {
                id: string;
            }

            model Timestamps {
                ...Base;
                createdAt: utcDateTime;
            }

            model Entity {
                name: string;
                ...Timestamps;
            }
        "#;

        let file = parse(source).unwrap();
        let models = build_model_map(&file);

        let entity = file.models().find(|m| m.name == "Entity").unwrap();
        let properties = resolve_properties(entity, &models);

        // Should have: name, id, createdAt
        assert_eq!(properties.len(), 3);

        let names: Vec<_> = properties.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"id"));
        assert!(names.contains(&"name"));
        assert!(names.contains(&"createdAt"));
    }

    #[test]
    fn test_enum_with_explicit_values() {
        let source = r#"
            enum TaskStatus {
                pending,
                inProgress: "in_progress",
                completed,
            }
        "#;

        let file = parse(source).unwrap();
        let enum_def = file.enums().next().unwrap();

        assert_eq!(enum_def.members.len(), 3);

        // Check that inProgress has explicit value
        let in_progress = enum_def.members.iter().find(|m| m.name == "inProgress").unwrap();
        assert!(matches!(&in_progress.value, Some(ast::Value::String(s)) if s == "in_progress"));
    }

    #[test]
    fn test_custom_scalar_parsing() {
        let source = r#"
            @format("uuid")
            scalar uuid extends string;

            model User {
                id: uuid;
            }
        "#;

        let file = parse(source).unwrap();

        // Check scalar
        let scalar = file.scalars().next().unwrap();
        assert_eq!(scalar.name, "uuid");
        assert_eq!(scalar.extends, Some("string".to_string()));

        // Check model uses the scalar
        let user = file.models().next().unwrap();
        let id_prop = &user.properties[0];
        assert!(matches!(&id_prop.type_ref, ast::TypeRef::Named(n) if n == "uuid"));
    }

    #[test]
    fn test_interface_with_decorators() {
        let source = r#"
            @route("/users")
            interface UserService {
                @get
                list(): User[];

                @get
                @route("/{id}")
                get(@path id: string): User;

                @post
                create(@body body: CreateUserRequest): User;
            }
        "#;

        let file = parse(source).unwrap();
        let iface = file.interfaces().next().unwrap();

        assert_eq!(iface.name, "UserService");
        assert_eq!(iface.operations.len(), 3);

        // Check route decorator
        let route_dec = iface.decorators.iter().find(|d| d.name == "route").unwrap();
        assert_eq!(route_dec.get_string_arg(0), Some("/users"));

        // Check operations
        let list_op = &iface.operations[0];
        assert_eq!(list_op.name, "list");
        assert!(list_op.decorators.iter().any(|d| d.name == "get"));

        let get_op = &iface.operations[1];
        assert_eq!(get_op.params.len(), 1);
        assert!(get_op.params[0].decorators.iter().any(|d| d.name == "path"));
    }

    #[test]
    fn test_generic_model() {
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
    fn test_union_type_in_property() {
        let source = r#"
            model Response {
                status: "healthy" | "degraded" | "unhealthy";
            }
        "#;

        let file = parse(source).unwrap();
        let model = file.models().next().unwrap();
        let prop = &model.properties[0];

        assert!(matches!(&prop.type_ref, ast::TypeRef::Union(variants) if variants.len() == 3));
    }

    #[test]
    fn test_string_literal_union_variants() {
        let source = r#"
            model Config {
                mode: "debug" | "release" | "test";
            }
        "#;

        let file = parse(source).unwrap();
        let model = file.models().next().unwrap();
        let prop = &model.properties[0];

        if let ast::TypeRef::Union(variants) = &prop.type_ref {
            let literals: Vec<_> = variants
                .iter()
                .filter_map(|v| match v {
                    ast::TypeRef::StringLiteral(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(literals, vec!["debug", "release", "test"]);
        } else {
            panic!("Expected union type");
        }
    }
}
