//! TypeSpec Lexer
//!
//! Tokenizes TypeSpec source files.

use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r"//[^\n]*")]
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
pub enum Token {
    // Keywords
    #[token("import")]
    Import,
    #[token("using")]
    Using,
    #[token("namespace")]
    Namespace,
    #[token("model")]
    Model,
    #[token("enum")]
    Enum,
    #[token("union")]
    Union,
    #[token("interface")]
    Interface,
    #[token("op")]
    Op,
    #[token("scalar")]
    Scalar,
    #[token("alias")]
    Alias,
    #[token("extends")]
    Extends,
    #[token("is")]
    Is,

    // Decorators
    #[regex(r"@[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_]*)*", |lex| lex.slice()[1..].to_string())]
    Decorator(String),

    // Literals
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    StringLit(String),

    #[regex(r"-?[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    IntLit(Option<i64>),

    #[regex(r"-?[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    FloatLit(Option<f64>),

    #[token("true")]
    True,
    #[token("false")]
    False,

    // Identifiers
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // Punctuation
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("<")]
    LAngle,
    #[token(">")]
    RAngle,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("...")]
    Spread,
    #[token("?")]
    Question,
    #[token("=")]
    Eq,
    #[token("|")]
    Pipe,
    #[token("&")]
    Amp,
}

pub fn tokenize(input: &str) -> Vec<(Token, std::ops::Range<usize>)> {
    Token::lexer(input)
        .spanned()
        .filter_map(|(tok, span)| tok.ok().map(|t| (t, span)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let input = r#"
            @route("/users")
            model User {
                id: string;
                name?: string;
            }
        "#;

        let tokens = tokenize(input);
        assert!(tokens.iter().any(|(t, _)| matches!(t, Token::Decorator(d) if d == "route")));
        assert!(tokens.iter().any(|(t, _)| matches!(t, Token::Model)));
        assert!(tokens.iter().any(|(t, _)| matches!(t, Token::Ident(s) if s == "User")));
    }
}
