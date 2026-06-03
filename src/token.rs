use logos::{Lexer, Logos};

#[derive(Default, Debug, Clone, PartialEq)]
pub enum TFlowLexerError {
    #[default]
    InvalidToken,
}

#[derive(Logos, Debug, PartialEq, Clone, Copy)]
#[logos(error = TFlowLexerError)]
#[logos(extras = ())]
#[logos(skip r"[ \t\r\n\f]+")] // Пробельные символы
#[logos(skip(r"//.*", allow_greedy = true))] // Однострочные комментарии
pub enum Token {
    // Ключевые слова
    #[token("struct")]
    Struct,
    #[token("const")]
    Const,
    #[token("table")]
    Table,
    #[token("param")]
    Param,
    #[token("state")]
    State,
    #[token("keep")]
    Keep,
    #[token("fn")]
    Fn,
    #[token("node")]
    Node,
    #[token("grid")]
    Grid,
    #[token("step")]
    Step,
    #[token("run")]
    Run,
    #[token("let")]
    Let,
    #[token("next")]
    Next,
    #[token("return")]
    Return,
    #[token("as")]
    As,

    // Логические константы и специальные идентификаторы
    #[token("true")]
    BoolTrue,
    #[token("false")]
    BoolFalse,
    #[token("now")]
    Now,
    #[token("prev")]
    Prev,

    // Системный контекст
    #[token("self")]
    SelfInfo,

    // Идентификаторы
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident,

    // Литералы

    // Целые числа
    #[regex(r"0x[0-9a-fA-F]+|0b[01]+|[0-9]+", parse_int)]
    IntLiteral(i64),

    // Числа с плавающей точкой
    #[regex(r"([0-9]+\.[0-9]*|[0-9]*\.[0-9]+)([eE][+-]?[0-9]+)?", parse_float)]
    FloatLiteral(f64),

    // Операторы и Спецсимволы
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,

    #[token("=")]
    Assign,
    #[token("+=")]
    AddAssign,
    #[token("-=")]
    SubAssign,
    #[token("*=")]
    MulAssign,
    #[token("/=")]
    DivAssign,

    #[token("==")]
    Eq,
    #[token("!=")]
    Neq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,

    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Not,

    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("@")]
    At,
    #[token("#")]
    Hash,

    // Скобки
    #[token("(")]
    ParenOpen,
    #[token(")")]
    ParenClose,
    #[token("[")]
    BracketOpen,
    #[token("]")]
    BracketClose,
    #[token("{")]
    BraceOpen,
    #[token("}")]
    BraceClose,
}

// Функции-хелперы для парсинга чисел прямо во время лексического анализа

fn parse_int(lex: &mut Lexer<Token>) -> Option<i64> {
    let slice = lex.slice();
    if let Some(stripped) = slice.strip_prefix("0x") {
        i64::from_str_radix(stripped, 16).ok()
    } else if let Some(stripped) = slice.strip_prefix("0b") {
        i64::from_str_radix(stripped, 2).ok()
    } else {
        slice.parse().ok()
    }
}

fn parse_float(lex: &mut Lexer<Token>) -> Option<f64> {
    lex.slice().parse().ok()
}
