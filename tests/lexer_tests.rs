//! Comprehensive lexer tests for TypeSpec tokenizer

use typespec_api::lexer::{tokenize, Token};

// ============================================================================
// Basic Token Tests
// ============================================================================

#[test]
fn test_tokenize_keywords() {
    let input = "import using namespace model enum union interface op scalar alias extends is";
    let tokens = tokenize(input);
    let token_types: Vec<_> = tokens.iter().map(|(t, _)| t.clone()).collect();

    assert!(matches!(token_types[0], Token::Import));
    assert!(matches!(token_types[1], Token::Using));
    assert!(matches!(token_types[2], Token::Namespace));
    assert!(matches!(token_types[3], Token::Model));
    assert!(matches!(token_types[4], Token::Enum));
    assert!(matches!(token_types[5], Token::Union));
    assert!(matches!(token_types[6], Token::Interface));
    assert!(matches!(token_types[7], Token::Op));
    assert!(matches!(token_types[8], Token::Scalar));
    assert!(matches!(token_types[9], Token::Alias));
    assert!(matches!(token_types[10], Token::Extends));
    assert!(matches!(token_types[11], Token::Is));
}

#[test]
fn test_tokenize_identifiers() {
    let input = "User UserService _private myVar123";
    let tokens = tokenize(input);

    assert!(matches!(&tokens[0].0, Token::Ident(s) if s == "User"));
    assert!(matches!(&tokens[1].0, Token::Ident(s) if s == "UserService"));
    assert!(matches!(&tokens[2].0, Token::Ident(s) if s == "_private"));
    assert!(matches!(&tokens[3].0, Token::Ident(s) if s == "myVar123"));
}

#[test]
fn test_tokenize_string_literals() {
    let input = r#""hello" "world with spaces" "escaped \"quotes\"""#;
    let tokens = tokenize(input);

    assert!(matches!(&tokens[0].0, Token::StringLit(s) if s == "hello"));
    assert!(matches!(&tokens[1].0, Token::StringLit(s) if s == "world with spaces"));
    assert!(matches!(&tokens[2].0, Token::StringLit(s) if s == r#"escaped \"quotes\""#));
}

#[test]
fn test_tokenize_number_literals() {
    let input = "42 -17 3.14 -2.5";
    let tokens = tokenize(input);

    assert!(matches!(&tokens[0].0, Token::IntLit(Some(42))));
    assert!(matches!(&tokens[1].0, Token::IntLit(Some(-17))));
    assert!(matches!(&tokens[2].0, Token::FloatLit(Some(f)) if (*f - 3.14).abs() < 0.001));
    assert!(matches!(&tokens[3].0, Token::FloatLit(Some(f)) if (*f + 2.5).abs() < 0.001));
}

#[test]
fn test_tokenize_boolean_literals() {
    let input = "true false";
    let tokens = tokenize(input);

    assert!(matches!(&tokens[0].0, Token::True));
    assert!(matches!(&tokens[1].0, Token::False));
}

// ============================================================================
// Decorator Tests
// ============================================================================

#[test]
fn test_tokenize_simple_decorators() {
    let input = "@route @get @post @put @delete @patch";
    let tokens = tokenize(input);

    assert!(matches!(&tokens[0].0, Token::Decorator(s) if s == "route"));
    assert!(matches!(&tokens[1].0, Token::Decorator(s) if s == "get"));
    assert!(matches!(&tokens[2].0, Token::Decorator(s) if s == "post"));
    assert!(matches!(&tokens[3].0, Token::Decorator(s) if s == "put"));
    assert!(matches!(&tokens[4].0, Token::Decorator(s) if s == "delete"));
    assert!(matches!(&tokens[5].0, Token::Decorator(s) if s == "patch"));
}

#[test]
fn test_tokenize_qualified_decorators() {
    let input = "@TypeSpec.Http.get @OpenAPI.info";
    let tokens = tokenize(input);

    assert!(matches!(&tokens[0].0, Token::Decorator(s) if s == "TypeSpec.Http.get"));
    assert!(matches!(&tokens[1].0, Token::Decorator(s) if s == "OpenAPI.info"));
}

// ============================================================================
// Punctuation Tests
// ============================================================================

#[test]
fn test_tokenize_punctuation() {
    let input = "{ } ( ) [ ] < > : ; , . ? = | &";
    let tokens = tokenize(input);
    let token_types: Vec<_> = tokens.iter().map(|(t, _)| t.clone()).collect();

    assert!(matches!(token_types[0], Token::LBrace));
    assert!(matches!(token_types[1], Token::RBrace));
    assert!(matches!(token_types[2], Token::LParen));
    assert!(matches!(token_types[3], Token::RParen));
    assert!(matches!(token_types[4], Token::LBracket));
    assert!(matches!(token_types[5], Token::RBracket));
    assert!(matches!(token_types[6], Token::LAngle));
    assert!(matches!(token_types[7], Token::RAngle));
    assert!(matches!(token_types[8], Token::Colon));
    assert!(matches!(token_types[9], Token::Semi));
    assert!(matches!(token_types[10], Token::Comma));
    assert!(matches!(token_types[11], Token::Dot));
    assert!(matches!(token_types[12], Token::Question));
    assert!(matches!(token_types[13], Token::Eq));
    assert!(matches!(token_types[14], Token::Pipe));
    assert!(matches!(token_types[15], Token::Amp));
}

#[test]
fn test_tokenize_spread_operator() {
    let input = "...Timestamps";
    let tokens = tokenize(input);

    assert!(matches!(&tokens[0].0, Token::Spread));
    assert!(matches!(&tokens[1].0, Token::Ident(s) if s == "Timestamps"));
}

// ============================================================================
// Comment Tests
// ============================================================================

#[test]
fn test_skip_line_comments() {
    let input = r#"model // this is a comment
User"#;
    let tokens = tokenize(input);

    assert_eq!(tokens.len(), 2);
    assert!(matches!(&tokens[0].0, Token::Model));
    assert!(matches!(&tokens[1].0, Token::Ident(s) if s == "User"));
}

#[test]
fn test_skip_block_comments() {
    let input = "model /* block comment */ User";
    let tokens = tokenize(input);

    assert_eq!(tokens.len(), 2);
    assert!(matches!(&tokens[0].0, Token::Model));
    assert!(matches!(&tokens[1].0, Token::Ident(s) if s == "User"));
}

#[test]
fn test_skip_multiline_block_comments() {
    let input = r#"model /*
    this is a
    multiline comment
    */ User"#;
    let tokens = tokenize(input);

    assert_eq!(tokens.len(), 2);
    assert!(matches!(&tokens[0].0, Token::Model));
    assert!(matches!(&tokens[1].0, Token::Ident(s) if s == "User"));
}

// ============================================================================
// Whitespace Tests
// ============================================================================

#[test]
fn test_skip_whitespace() {
    let input = "  model   \t\n  User  \r\n  ";
    let tokens = tokenize(input);

    assert_eq!(tokens.len(), 2);
    assert!(matches!(&tokens[0].0, Token::Model));
    assert!(matches!(&tokens[1].0, Token::Ident(s) if s == "User"));
}

// ============================================================================
// Complex Token Sequences
// ============================================================================

#[test]
fn test_tokenize_model_definition() {
    let input = r#"
        model User {
            id: string;
            name?: string;
            age: int32;
        }
    "#;
    let tokens = tokenize(input);
    let token_types: Vec<_> = tokens.iter().map(|(t, _)| t.clone()).collect();

    assert!(matches!(token_types[0], Token::Model));
    assert!(matches!(&token_types[1], Token::Ident(s) if s == "User"));
    assert!(matches!(token_types[2], Token::LBrace));
    assert!(matches!(&token_types[3], Token::Ident(s) if s == "id"));
    assert!(matches!(token_types[4], Token::Colon));
    assert!(matches!(&token_types[5], Token::Ident(s) if s == "string"));
    assert!(matches!(token_types[6], Token::Semi));
}

#[test]
fn test_tokenize_interface_definition() {
    let input = r#"
        @route("/users")
        interface UserService {
            @get
            list(): User[];
        }
    "#;
    let tokens = tokenize(input);

    assert!(tokens
        .iter()
        .any(|(t, _)| matches!(t, Token::Decorator(s) if s == "route")));
    assert!(tokens
        .iter()
        .any(|(t, _)| matches!(t, Token::StringLit(s) if s == "/users")));
    assert!(tokens.iter().any(|(t, _)| matches!(t, Token::Interface)));
    assert!(tokens
        .iter()
        .any(|(t, _)| matches!(t, Token::Decorator(s) if s == "get")));
}

#[test]
fn test_tokenize_generic_types() {
    let input = "PaginatedResponse<User>";
    let tokens = tokenize(input);

    assert!(matches!(&tokens[0].0, Token::Ident(s) if s == "PaginatedResponse"));
    assert!(matches!(&tokens[1].0, Token::LAngle));
    assert!(matches!(&tokens[2].0, Token::Ident(s) if s == "User"));
    assert!(matches!(&tokens[3].0, Token::RAngle));
}

#[test]
fn test_tokenize_union_types() {
    let input = r#""success" | "error" | "pending""#;
    let tokens = tokenize(input);

    assert!(matches!(&tokens[0].0, Token::StringLit(s) if s == "success"));
    assert!(matches!(&tokens[1].0, Token::Pipe));
    assert!(matches!(&tokens[2].0, Token::StringLit(s) if s == "error"));
    assert!(matches!(&tokens[3].0, Token::Pipe));
    assert!(matches!(&tokens[4].0, Token::StringLit(s) if s == "pending"));
}

#[test]
fn test_tokenize_array_types() {
    let input = "string[] int32[][]";
    let tokens = tokenize(input);

    assert!(matches!(&tokens[0].0, Token::Ident(s) if s == "string"));
    assert!(matches!(&tokens[1].0, Token::LBracket));
    assert!(matches!(&tokens[2].0, Token::RBracket));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_input() {
    let tokens = tokenize("");
    assert!(tokens.is_empty());
}

#[test]
fn test_only_whitespace() {
    let tokens = tokenize("   \t\n\r  ");
    assert!(tokens.is_empty());
}

#[test]
fn test_only_comments() {
    let tokens = tokenize("// just a comment");
    assert!(tokens.is_empty());
}

#[test]
fn test_token_spans() {
    let input = "model User";
    let tokens = tokenize(input);

    // "model" spans 0..5
    assert_eq!(tokens[0].1.start, 0);
    assert_eq!(tokens[0].1.end, 5);

    // "User" spans 6..10
    assert_eq!(tokens[1].1.start, 6);
    assert_eq!(tokens[1].1.end, 10);
}
