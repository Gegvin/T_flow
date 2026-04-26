use logos::Logos;
// ФАЙЛ ЗАГЛУШКА
#[derive(Debug, PartialEq, Logos)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token {
    #[token("const")]
    Const,

    #[token("param")]
    Param,

    #[token("state")]
    State,

    #[token("node")]
    Node,

    #[token("fn")]
    Fn,

    #[token("grid")]
    Grid,

    #[token("step")]
    Step,

    #[token("run")]
    Run,

    #[token("let")]
    Let,

    #[token("return")]
    Return,

    #[token("next")]
    Next,

    #[token("=")]
    Eq,

    #[token(";")]
    Semicolon,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[regex("[0-9]+")]
    Int,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident,
}
