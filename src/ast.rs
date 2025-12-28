//! TypeSpec AST
//!
//! Abstract syntax tree for TypeSpec definitions.

use std::collections::HashMap;

/// Root of a TypeSpec file.
#[derive(Debug, Clone, Default)]
pub struct TypeSpecFile {
    pub imports: Vec<Import>,
    pub usings: Vec<Using>,
    pub namespace: Option<String>,
    pub declarations: Vec<Declaration>,
}

impl TypeSpecFile {
    /// Get all models.
    pub fn models(&self) -> impl Iterator<Item = &Model> {
        self.declarations.iter().filter_map(|d| match d {
            Declaration::Model(m) => Some(m),
            _ => None,
        })
    }

    /// Get all enums.
    pub fn enums(&self) -> impl Iterator<Item = &Enum> {
        self.declarations.iter().filter_map(|d| match d {
            Declaration::Enum(e) => Some(e),
            _ => None,
        })
    }

    /// Get all interfaces (services).
    pub fn interfaces(&self) -> impl Iterator<Item = &Interface> {
        self.declarations.iter().filter_map(|d| match d {
            Declaration::Interface(i) => Some(i),
            _ => None,
        })
    }

    /// Get all scalars.
    pub fn scalars(&self) -> impl Iterator<Item = &Scalar> {
        self.declarations.iter().filter_map(|d| match d {
            Declaration::Scalar(s) => Some(s),
            _ => None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct Using {
    pub namespace: String,
}

#[derive(Debug, Clone)]
pub enum Declaration {
    Model(Model),
    Enum(Enum),
    Union(Union),
    Interface(Interface),
    Scalar(Scalar),
    Alias(Alias),
    Namespace(Namespace),
}

#[derive(Debug, Clone)]
pub struct Namespace {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub type_params: Vec<String>,
    pub extends: Option<TypeRef>,
    pub properties: Vec<Property>,
    pub spread_refs: Vec<TypeRef>,
}

#[derive(Debug, Clone)]
pub struct Property {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub type_ref: TypeRef,
    pub optional: bool,
    pub default: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub members: Vec<EnumMember>,
}

#[derive(Debug, Clone)]
pub struct EnumMember {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub value: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct Union {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub variants: Vec<UnionVariant>,
}

#[derive(Debug, Clone)]
pub struct UnionVariant {
    pub name: Option<String>,
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub params: Vec<OperationParam>,
    pub return_type: Option<TypeRef>,
}

#[derive(Debug, Clone)]
pub struct OperationParam {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub type_ref: TypeRef,
    pub optional: bool,
    pub spread: bool,
}

#[derive(Debug, Clone)]
pub struct Scalar {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub extends: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone)]
pub struct Decorator {
    pub name: String,
    pub args: Vec<DecoratorArg>,
}

impl Decorator {
    pub fn get_string_arg(&self, index: usize) -> Option<&str> {
        self.args.get(index).and_then(|a| match a {
            DecoratorArg::Value(Value::String(s)) => Some(s.as_str()),
            _ => None,
        })
    }
}

#[derive(Debug, Clone)]
pub enum DecoratorArg {
    Value(Value),
    Named { name: String, value: Value },
}

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Ident(String),
    /// Qualified identifier like TaskStatus.pending
    QualifiedIdent(Vec<String>),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

#[derive(Debug, Clone)]
pub enum TypeRef {
    /// Built-in type: string, int32, boolean, etc.
    Builtin(String),

    /// Reference to a named type
    Named(String),

    /// Qualified name: Namespace.Type
    Qualified(Vec<String>),

    /// Array type: Type[]
    Array(Box<TypeRef>),

    /// Generic type: Type<T, U>
    Generic {
        base: Box<TypeRef>,
        args: Vec<TypeRef>,
    },

    /// Union type: A | B
    Union(Vec<TypeRef>),

    /// Intersection type: A & B
    Intersection(Vec<TypeRef>),

    /// Optional type (for return types)
    Optional(Box<TypeRef>),

    /// Literal string type
    StringLiteral(String),

    /// Literal integer type (for status codes like 200, 404)
    IntLiteral(i64),

    /// Anonymous model
    AnonymousModel(Vec<Property>),
}

impl TypeRef {
    /// Check if this is a builtin primitive type.
    pub fn is_primitive(&self) -> bool {
        matches!(self, TypeRef::Builtin(_))
    }

    /// Get the base type name (without generics).
    pub fn base_name(&self) -> Option<&str> {
        match self {
            TypeRef::Builtin(n) | TypeRef::Named(n) => Some(n),
            TypeRef::Generic { base, .. } => base.base_name(),
            _ => None,
        }
    }
}
