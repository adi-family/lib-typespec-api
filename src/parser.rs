//! TypeSpec Parser
//!
//! Parses tokenized TypeSpec into AST.

use crate::ast::*;
use crate::lexer::Token;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token at position {pos}: expected {expected}, got {got:?}")]
    UnexpectedToken {
        pos: usize,
        expected: String,
        got: Option<Token>,
    },

    #[error("Unexpected end of input")]
    UnexpectedEof,

    #[error("Invalid syntax: {0}")]
    InvalidSyntax(String),
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        match self.peek() {
            Some(tok) if tok == expected => {
                self.advance();
                Ok(())
            }
            other => Err(ParseError::UnexpectedToken {
                pos: self.pos,
                expected: format!("{:?}", expected),
                got: other.cloned(),
            }),
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s),
            // Allow keywords to be used as identifiers (property names, etc.)
            Some(Token::Model) => Ok("model".to_string()),
            Some(Token::Enum) => Ok("enum".to_string()),
            Some(Token::Union) => Ok("union".to_string()),
            Some(Token::Interface) => Ok("interface".to_string()),
            Some(Token::Scalar) => Ok("scalar".to_string()),
            Some(Token::Alias) => Ok("alias".to_string()),
            Some(Token::Namespace) => Ok("namespace".to_string()),
            Some(Token::Import) => Ok("import".to_string()),
            Some(Token::Using) => Ok("using".to_string()),
            Some(Token::Extends) => Ok("extends".to_string()),
            Some(Token::Is) => Ok("is".to_string()),
            Some(Token::Op) => Ok("op".to_string()),
            other => Err(ParseError::UnexpectedToken {
                pos: self.pos,
                expected: "identifier".to_string(),
                got: other,
            }),
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Some(Token::StringLit(s)) => Ok(s),
            other => Err(ParseError::UnexpectedToken {
                pos: self.pos,
                expected: "string literal".to_string(),
                got: other,
            }),
        }
    }

    pub fn parse_file(&mut self) -> Result<TypeSpecFile, ParseError> {
        let mut file = TypeSpecFile::default();

        while self.peek().is_some() {
            // Collect decorators
            let decorators = self.parse_decorators()?;

            match self.peek() {
                Some(Token::Import) => {
                    file.imports.push(self.parse_import()?);
                }
                Some(Token::Using) => {
                    file.usings.push(self.parse_using()?);
                }
                Some(Token::Namespace) => {
                    self.advance();
                    let name = self.parse_qualified_name()?;

                    // Check if this is a simple namespace declaration (with ;)
                    // or a namespace block (with {})
                    if self.peek() == Some(&Token::Semi) {
                        // Top-level namespace declaration: namespace Name;
                        self.advance();
                        file.namespace = Some(name);
                        // Note: decorators on top-level namespace are ignored for now
                    } else {
                        // Nested namespace block: namespace Name { ... }
                        self.expect(&Token::LBrace)?;
                        let mut declarations = Vec::new();
                        while self.peek() != Some(&Token::RBrace) {
                            let decs = self.parse_decorators()?;
                            match self.peek() {
                                Some(Token::Model) => {
                                    declarations.push(Declaration::Model(self.parse_model(decs)?));
                                }
                                Some(Token::Enum) => {
                                    declarations.push(Declaration::Enum(self.parse_enum(decs)?));
                                }
                                Some(Token::Interface) => {
                                    declarations.push(Declaration::Interface(self.parse_interface(decs)?));
                                }
                                _ => break,
                            }
                        }
                        self.expect(&Token::RBrace)?;
                        file.declarations.push(Declaration::Namespace(Namespace {
                            name,
                            decorators,
                            declarations,
                        }));
                    }
                }
                Some(Token::Model) => {
                    file.declarations
                        .push(Declaration::Model(self.parse_model(decorators)?));
                }
                Some(Token::Enum) => {
                    file.declarations
                        .push(Declaration::Enum(self.parse_enum(decorators)?));
                }
                Some(Token::Union) => {
                    file.declarations
                        .push(Declaration::Union(self.parse_union(decorators)?));
                }
                Some(Token::Interface) => {
                    file.declarations
                        .push(Declaration::Interface(self.parse_interface(decorators)?));
                }
                Some(Token::Scalar) => {
                    file.declarations
                        .push(Declaration::Scalar(self.parse_scalar(decorators)?));
                }
                Some(Token::Alias) => {
                    file.declarations
                        .push(Declaration::Alias(self.parse_alias()?));
                }
                Some(_) => {
                    return Err(ParseError::InvalidSyntax(format!(
                        "Unexpected token: {:?}",
                        self.peek()
                    )));
                }
                None => break,
            }
        }

        Ok(file)
    }

    fn parse_decorators(&mut self) -> Result<Vec<Decorator>, ParseError> {
        let mut decorators = Vec::new();

        while let Some(Token::Decorator(name)) = self.peek().cloned() {
            self.advance();
            let args = if self.peek() == Some(&Token::LParen) {
                self.parse_decorator_args()?
            } else {
                Vec::new()
            };
            decorators.push(Decorator { name, args });
        }

        Ok(decorators)
    }

    fn parse_decorator_args(&mut self) -> Result<Vec<DecoratorArg>, ParseError> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();

        while self.peek() != Some(&Token::RParen) {
            // Check for named argument
            let is_named = {
                let has_ident = matches!(self.peek(), Some(Token::Ident(_)));
                has_ident && self.tokens.get(self.pos + 1) == Some(&Token::Colon)
            };

            if is_named {
                let name = self.expect_ident()?;
                self.expect(&Token::Colon)?;
                let value = self.parse_value()?;
                args.push(DecoratorArg::Named { name, value });
            } else {
                let value = self.parse_value()?;
                args.push(DecoratorArg::Value(value));
            }

            if self.peek() == Some(&Token::Comma) {
                self.advance();
            }
        }

        self.expect(&Token::RParen)?;
        Ok(args)
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        match self.peek().cloned() {
            Some(Token::StringLit(s)) => {
                self.advance();
                Ok(Value::String(s))
            }
            Some(Token::IntLit(Some(n))) => {
                self.advance();
                Ok(Value::Int(n))
            }
            Some(Token::FloatLit(Some(n))) => {
                self.advance();
                Ok(Value::Float(n))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Value::Bool(true))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Value::Bool(false))
            }
            Some(Token::Ident(s)) => {
                self.advance();
                // Handle qualified names like TaskStatus.pending
                let mut parts = vec![s];
                while self.peek() == Some(&Token::Dot) {
                    self.advance();
                    parts.push(self.expect_ident()?);
                }
                if parts.len() > 1 {
                    Ok(Value::QualifiedIdent(parts))
                } else {
                    Ok(Value::Ident(parts.into_iter().next().unwrap()))
                }
            }
            Some(Token::LBracket) => {
                self.advance();
                let mut items = Vec::new();
                while self.peek() != Some(&Token::RBracket) {
                    items.push(self.parse_value()?);
                    if self.peek() == Some(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Value::Array(items))
            }
            Some(Token::LBrace) => {
                self.advance();
                let mut map = HashMap::new();
                while self.peek() != Some(&Token::RBrace) {
                    let key = self.expect_ident()?;
                    self.expect(&Token::Colon)?;
                    let value = self.parse_value()?;
                    map.insert(key, value);
                    if self.peek() == Some(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBrace)?;
                Ok(Value::Object(map))
            }
            other => Err(ParseError::UnexpectedToken {
                pos: self.pos,
                expected: "value".to_string(),
                got: other,
            }),
        }
    }

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        self.expect(&Token::Import)?;
        let path = self.expect_string()?;
        self.expect(&Token::Semi)?;
        Ok(Import { path })
    }

    fn parse_using(&mut self) -> Result<Using, ParseError> {
        self.expect(&Token::Using)?;
        let namespace = self.parse_qualified_name()?;
        self.expect(&Token::Semi)?;
        Ok(Using { namespace })
    }

    fn parse_qualified_name(&mut self) -> Result<String, ParseError> {
        let mut parts = vec![self.expect_ident()?];
        while self.peek() == Some(&Token::Dot) {
            self.advance();
            parts.push(self.expect_ident()?);
        }
        Ok(parts.join("."))
    }

    fn parse_model(&mut self, decorators: Vec<Decorator>) -> Result<Model, ParseError> {
        self.expect(&Token::Model)?;
        let name = self.expect_ident()?;

        // Type parameters
        let type_params = if self.peek() == Some(&Token::LAngle) {
            self.parse_type_params()?
        } else {
            Vec::new()
        };

        // Extends
        let extends = if self.peek() == Some(&Token::Extends) {
            self.advance();
            Some(self.parse_type_ref()?)
        } else {
            None
        };

        self.expect(&Token::LBrace)?;

        let mut properties = Vec::new();
        let mut spread_refs = Vec::new();

        while self.peek() != Some(&Token::RBrace) {
            let prop_decorators = self.parse_decorators()?;

            // Check for spread operator
            if self.peek() == Some(&Token::Spread) {
                self.advance();
                spread_refs.push(self.parse_type_ref()?);
                if self.peek() == Some(&Token::Semi) {
                    self.advance();
                }
                continue;
            }

            let prop_name = self.expect_ident()?;
            let optional = if self.peek() == Some(&Token::Question) {
                self.advance();
                true
            } else {
                false
            };

            self.expect(&Token::Colon)?;
            let type_ref = self.parse_type_ref()?;

            let default = if self.peek() == Some(&Token::Eq) {
                self.advance();
                Some(self.parse_value()?)
            } else {
                None
            };

            if self.peek() == Some(&Token::Semi) {
                self.advance();
            }

            properties.push(Property {
                name: prop_name,
                decorators: prop_decorators,
                type_ref,
                optional,
                default,
            });
        }

        self.expect(&Token::RBrace)?;

        Ok(Model {
            name,
            decorators,
            type_params,
            extends,
            properties,
            spread_refs,
        })
    }

    fn parse_type_params(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(&Token::LAngle)?;
        let mut params = Vec::new();

        while self.peek() != Some(&Token::RAngle) {
            params.push(self.expect_ident()?);
            if self.peek() == Some(&Token::Comma) {
                self.advance();
            }
        }

        self.expect(&Token::RAngle)?;
        Ok(params)
    }

    fn parse_type_ref(&mut self) -> Result<TypeRef, ParseError> {
        let mut type_ref = self.parse_primary_type()?;

        // Handle union types (A | B)
        if self.peek() == Some(&Token::Pipe) {
            let mut variants = vec![type_ref];
            while self.peek() == Some(&Token::Pipe) {
                self.advance();
                variants.push(self.parse_primary_type()?);
            }
            type_ref = TypeRef::Union(variants);
        }

        // Handle intersection types (A & B)
        if self.peek() == Some(&Token::Amp) {
            let mut parts = vec![type_ref];
            while self.peek() == Some(&Token::Amp) {
                self.advance();
                parts.push(self.parse_primary_type()?);
            }
            type_ref = TypeRef::Intersection(parts);
        }

        Ok(type_ref)
    }

    fn parse_primary_type(&mut self) -> Result<TypeRef, ParseError> {
        let base = match self.peek().cloned() {
            Some(Token::Ident(name)) => {
                self.advance();

                // Check for qualified name
                let mut parts = vec![name];
                while self.peek() == Some(&Token::Dot) {
                    self.advance();
                    parts.push(self.expect_ident()?);
                }

                let base_type = if parts.len() > 1 {
                    TypeRef::Qualified(parts)
                } else {
                    let name = parts.into_iter().next().unwrap();
                    if is_builtin(&name) {
                        TypeRef::Builtin(name)
                    } else {
                        TypeRef::Named(name)
                    }
                };

                // Check for generic args
                if self.peek() == Some(&Token::LAngle) {
                    self.expect(&Token::LAngle)?;
                    let mut args = Vec::new();
                    while self.peek() != Some(&Token::RAngle) {
                        args.push(self.parse_type_ref()?);
                        if self.peek() == Some(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RAngle)?;
                    TypeRef::Generic {
                        base: Box::new(base_type),
                        args,
                    }
                } else {
                    base_type
                }
            }
            Some(Token::StringLit(s)) => {
                self.advance();
                TypeRef::StringLiteral(s)
            }
            Some(Token::IntLit(Some(n))) => {
                self.advance();
                TypeRef::IntLiteral(n)
            }
            Some(Token::LBrace) => {
                // Anonymous model
                self.advance();
                let mut properties = Vec::new();
                while self.peek() != Some(&Token::RBrace) {
                    let decorators = self.parse_decorators()?;
                    let name = self.expect_ident()?;
                    let optional = if self.peek() == Some(&Token::Question) {
                        self.advance();
                        true
                    } else {
                        false
                    };
                    self.expect(&Token::Colon)?;
                    let type_ref = self.parse_type_ref()?;
                    if self.peek() == Some(&Token::Semi) {
                        self.advance();
                    }
                    properties.push(Property {
                        name,
                        decorators,
                        type_ref,
                        optional,
                        default: None,
                    });
                }
                self.expect(&Token::RBrace)?;
                TypeRef::AnonymousModel(properties)
            }
            other => {
                return Err(ParseError::UnexpectedToken {
                    pos: self.pos,
                    expected: "type".to_string(),
                    got: other,
                })
            }
        };

        // Handle array suffix []
        let mut result = base;
        while self.peek() == Some(&Token::LBracket) {
            self.advance();
            self.expect(&Token::RBracket)?;
            result = TypeRef::Array(Box::new(result));
        }

        Ok(result)
    }

    fn parse_enum(&mut self, decorators: Vec<Decorator>) -> Result<Enum, ParseError> {
        self.expect(&Token::Enum)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut members = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            let member_decorators = self.parse_decorators()?;
            let member_name = self.expect_ident()?;

            let value = if self.peek() == Some(&Token::Colon) {
                self.advance();
                Some(self.parse_value()?)
            } else {
                None
            };

            if self.peek() == Some(&Token::Comma) {
                self.advance();
            }

            members.push(EnumMember {
                name: member_name,
                decorators: member_decorators,
                value,
            });
        }

        self.expect(&Token::RBrace)?;

        Ok(Enum {
            name,
            decorators,
            members,
        })
    }

    fn parse_union(&mut self, decorators: Vec<Decorator>) -> Result<Union, ParseError> {
        self.expect(&Token::Union)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut variants = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            // Named variant
            if matches!(self.peek(), Some(Token::Ident(_))) {
                let variant_name = self.expect_ident()?;
                self.expect(&Token::Colon)?;
                let type_ref = self.parse_type_ref()?;
                variants.push(UnionVariant {
                    name: Some(variant_name),
                    type_ref,
                });
            } else {
                // Anonymous variant (just type)
                let type_ref = self.parse_type_ref()?;
                variants.push(UnionVariant {
                    name: None,
                    type_ref,
                });
            }

            if self.peek() == Some(&Token::Comma) {
                self.advance();
            }
        }

        self.expect(&Token::RBrace)?;

        Ok(Union {
            name,
            decorators,
            variants,
        })
    }

    fn parse_interface(&mut self, decorators: Vec<Decorator>) -> Result<Interface, ParseError> {
        self.expect(&Token::Interface)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut operations = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            let op_decorators = self.parse_decorators()?;
            let op_name = self.expect_ident()?;

            self.expect(&Token::LParen)?;
            let mut params = Vec::new();

            while self.peek() != Some(&Token::RParen) {
                let param_decorators = self.parse_decorators()?;
                let spread = if self.peek() == Some(&Token::Spread) {
                    self.advance();
                    true
                } else {
                    false
                };

                // For spread types like ...PaginationParams, there's no name:type syntax
                // Check if next token after ident is colon or not
                let is_named_param = !spread || {
                    // Look ahead: if we have `name:` it's a named param, otherwise anonymous spread
                    self.tokens.get(self.pos + 1) == Some(&Token::Colon)
                        || self.tokens.get(self.pos + 1) == Some(&Token::Question)
                };

                let (param_name, optional, type_ref) = if is_named_param {
                    let name = self.expect_ident()?;
                    let opt = if self.peek() == Some(&Token::Question) {
                        self.advance();
                        true
                    } else {
                        false
                    };
                    self.expect(&Token::Colon)?;
                    let tr = self.parse_type_ref()?;
                    (name, opt, tr)
                } else {
                    // Anonymous spread: ...TypeName
                    let tr = self.parse_type_ref()?;
                    // Use empty string as placeholder name for anonymous spread
                    (String::new(), false, tr)
                };

                params.push(OperationParam {
                    name: param_name,
                    decorators: param_decorators,
                    type_ref,
                    optional,
                    spread,
                });

                if self.peek() == Some(&Token::Comma) {
                    self.advance();
                }
            }

            self.expect(&Token::RParen)?;

            let return_type = if self.peek() == Some(&Token::Colon) {
                self.advance();
                Some(self.parse_type_ref()?)
            } else {
                None
            };

            if self.peek() == Some(&Token::Semi) {
                self.advance();
            }

            operations.push(Operation {
                name: op_name,
                decorators: op_decorators,
                params,
                return_type,
            });
        }

        self.expect(&Token::RBrace)?;

        Ok(Interface {
            name,
            decorators,
            operations,
        })
    }

    fn parse_scalar(&mut self, decorators: Vec<Decorator>) -> Result<Scalar, ParseError> {
        self.expect(&Token::Scalar)?;
        let name = self.expect_ident()?;

        let extends = if self.peek() == Some(&Token::Extends) {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };

        self.expect(&Token::Semi)?;

        Ok(Scalar {
            name,
            decorators,
            extends,
        })
    }

    fn parse_alias(&mut self) -> Result<Alias, ParseError> {
        self.expect(&Token::Alias)?;
        let name = self.expect_ident()?;
        self.expect(&Token::Eq)?;
        let type_ref = self.parse_type_ref()?;
        self.expect(&Token::Semi)?;

        Ok(Alias { name, type_ref })
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "string"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "float32"
            | "float64"
            | "boolean"
            | "bytes"
            | "plainDate"
            | "plainTime"
            | "utcDateTime"
            | "offsetDateTime"
            | "duration"
            | "url"
            | "null"
            | "void"
            | "never"
            | "unknown"
    )
}

/// Parse TypeSpec source code.
pub fn parse(source: &str) -> Result<TypeSpecFile, ParseError> {
    let tokens: Vec<Token> = crate::lexer::tokenize(source)
        .into_iter()
        .map(|(t, _)| t)
        .collect();

    let mut parser = Parser::new(tokens);
    parser.parse_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model() {
        let source = r#"
            model User {
                id: string;
                name?: string;
                age: int32;
            }
        "#;

        let file = parse(source).unwrap();
        assert_eq!(file.models().count(), 1);
        let model = file.models().next().unwrap();
        assert_eq!(model.name, "User");
        assert_eq!(model.properties.len(), 3);
    }

    #[test]
    fn test_parse_interface() {
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
        assert_eq!(file.interfaces().count(), 1);
        let iface = file.interfaces().next().unwrap();
        assert_eq!(iface.name, "UserService");
        assert_eq!(iface.operations.len(), 3);
    }
}
