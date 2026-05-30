use crate::lexer_runner::LocatedToken;
use crate::token::Token;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

pub struct Parser<'a> {
    tokens: &'a [LocatedToken<Token>],
    pos: usize,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [LocatedToken<Token>]) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Vec<ParseError> {
        while !self.at_end() {
            self.parse_declaration();
        }
        self.errors
    }

    // ── утилиты ──────────────────────────────────────────────────────────

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.kind)
    }

    fn peek_tok(&self) -> Option<&LocatedToken<Token>> {
        self.tokens.get(self.pos)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn advance(&mut self) -> Option<&LocatedToken<Token>> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &Token) -> bool {
        self.peek().map(|t| t == kind).unwrap_or(false)
    }

    fn eat(&mut self, kind: &Token) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &Token, msg: &str) -> bool {
        if self.eat(kind) {
            true
        } else {
            self.error(msg);
            false
        }
    }

    fn error(&mut self, msg: &str) {
        let (line, col) = self.current_pos();
        self.errors.push(ParseError {
            message: msg.to_string(),
            line,
            column: col,
        });
    }

    fn current_pos(&self) -> (usize, usize) {
        self.tokens
            .get(self.pos)
            .map(|t| (t.location.start.line, t.location.start.column))
            .or_else(|| {
                self.tokens
                    .last()
                    .map(|t| (t.location.end.line, t.location.end.column))
            })
            .unwrap_or((1, 1))
    }

    fn synchronize(&mut self, stops: &[Token]) {
        while !self.at_end() {
            if let Some(t) = self.peek()
                && stops.contains(t)
            {
                return;
            }
            self.advance();
        }
    }

    // ── верхний уровень ──────────────────────────────────────────────────

    fn parse_declaration(&mut self) {
        match self.peek() {
            Some(Token::Struct) => self.parse_struct(),
            Some(Token::Const) => self.parse_const(),
            Some(Token::Table) => self.parse_table_or_param(Token::Table),
            Some(Token::Param) => self.parse_table_or_param(Token::Param),
            Some(Token::State) => self.parse_state(),
            Some(Token::Fn) => self.parse_fn(),
            Some(Token::Node) => self.parse_node(),
            Some(Token::Grid) => self.parse_grid(),
            Some(Token::Step) => self.parse_step(),
            _ => {
                self.error(&format!(
                    "unexpected token '{}': expected declaration (struct, const, table, param, state, fn, node, grid, step)",
                    self.peek_tok().map(|t| t.lexeme.as_str()).unwrap_or("<EOF>")
                ));
                self.advance();
                self.synchronize(&[
                    Token::Struct,
                    Token::Const,
                    Token::Table,
                    Token::Param,
                    Token::State,
                    Token::Fn,
                    Token::Node,
                    Token::Grid,
                    Token::Step,
                ]);
            }
        }
    }

    // ── struct ───────────────────────────────────────────────────────────

    fn parse_struct(&mut self) {
        self.advance();
        if !self.expect_ident("expected struct name after 'struct'") {
            return;
        }
        if !self.expect(&Token::BraceOpen, "expected '{' after struct name") {
            return;
        }
        while !self.check(&Token::BraceClose) && !self.at_end() {
            if !self.expect_ident("expected field name") {
                break;
            }
            if !self.expect(&Token::Colon, "expected ':' after field name") {
                break;
            }
            if !self.parse_type() {
                break;
            }
            self.eat(&Token::Comma);
        }
        self.expect(&Token::BraceClose, "expected '}' to close struct");
    }

    // ── const ────────────────────────────────────────────────────────────

    fn parse_const(&mut self) {
        self.advance();
        if !self.expect_ident("expected constant name after 'const'") {
            return;
        }
        if !self.expect(&Token::Assign, "expected '=' after constant name") {
            return;
        }
        self.parse_literal_or_error("expected literal value in const declaration");
        self.expect(&Token::Semicolon, "expected ';' after const declaration");
    }

    // ── table / param ────────────────────────────────────────────────────

    fn parse_table_or_param(&mut self, keyword: Token) {
        self.advance();
        let kw = if keyword == Token::Table {
            "table"
        } else {
            "param"
        };
        if !self.expect_ident(&format!("expected name after '{kw}'")) {
            return;
        }
        if !self.expect(&Token::Hash, &format!("expected '#' after {kw} name")) {
            return;
        }
        if !self.parse_bound() {
            return;
        }
        if !self.expect(&Token::Colon, "expected ':' before type") {
            return;
        }
        if !self.parse_type() {
            return;
        }
        while self.check(&Token::BracketOpen) {
            self.advance();
            if matches!(self.peek(), Some(Token::IntLiteral(_))) {
                self.advance();
            } else {
                self.error("expected integer size in dimension");
            }
            self.expect(&Token::BracketClose, "expected ']' after dimension size");
        }
        if !self.expect(&Token::Assign, "expected '=' before array literal") {
            return;
        }
        self.parse_array_literal();
        self.expect(
            &Token::Semicolon,
            &format!("expected ';' after {kw} declaration"),
        );
    }

    // ── state ────────────────────────────────────────────────────────────

    fn parse_state(&mut self) {
        self.advance();
        if !self.expect_ident("expected state name") {
            return;
        }
        if !self.expect(&Token::Colon, "expected ':' after state name") {
            return;
        }
        if !self.parse_type() {
            return;
        }
        if !self.expect(&Token::Keep, "expected 'keep' after type in state") {
            return;
        }
        if !self.expect(&Token::ParenOpen, "expected '(' after 'keep'") {
            return;
        }
        if matches!(self.peek(), Some(Token::IntLiteral(_))) {
            self.advance();
        } else {
            self.error("expected integer history depth in keep(...)");
        }
        if !self.expect(&Token::ParenClose, "expected ')' after history depth") {
            return;
        }
        if !self.expect(&Token::Assign, "expected '=' after keep(...)") {
            return;
        }
        if self.check(&Token::BracketOpen) {
            self.parse_array_literal();
        } else {
            self.parse_literal_or_error("expected literal or array as initial value");
        }
        if !self.expect(&Token::At, "expected '@' after initial values") {
            return;
        }
        self.parse_array_literal();
        if !self.expect(&Token::Hash, "expected '#' after flat array") {
            return;
        }
        self.parse_bound();
        self.expect(&Token::Semicolon, "expected ';' after state declaration");
    }

    // ── fn ───────────────────────────────────────────────────────────────

    fn parse_fn(&mut self) {
        self.advance();
        if !self.expect_ident("expected function name after 'fn'") {
            return;
        }
        if !self.expect(&Token::ParenOpen, "expected '(' after function name") {
            return;
        }
        self.parse_params();
        if !self.expect(&Token::ParenClose, "expected ')' after parameters") {
            return;
        }
        if !self.expect(&Token::Minus, "expected '->' after ')'") {
            return;
        }
        if !self.expect(&Token::Gt, "expected '>' in '->'") {
            return;
        }
        if !self.parse_type() {
            return;
        }
        if !self.expect(&Token::BraceOpen, "expected '{' to open function body") {
            return;
        }
        while !self.check(&Token::BraceClose) && !self.at_end() {
            self.parse_statement();
        }
        self.expect(&Token::BraceClose, "expected '}' to close function body");
    }

    // ── node ─────────────────────────────────────────────────────────────

    fn parse_node(&mut self) {
        self.advance();
        if !self.expect_ident("expected node name after 'node'") {
            return;
        }
        if !self.expect(&Token::ParenOpen, "expected '(' after node name") {
            return;
        }
        self.parse_params();
        if !self.expect(&Token::ParenClose, "expected ')' after parameters") {
            return;
        }
        if !self.expect(&Token::BraceOpen, "expected '{' to open node body") {
            return;
        }
        while !self.check(&Token::BraceClose) && !self.at_end() {
            match self.peek() {
                Some(Token::Let) => self.parse_let(),
                Some(Token::Next) => self.parse_next(),
                _ => {
                    self.error(&format!(
                        "unexpected '{}' in node body: only 'let' and 'next' are allowed",
                        self.peek_tok()
                            .map(|t| t.lexeme.as_str())
                            .unwrap_or("<EOF>")
                    ));
                    self.advance();
                    self.synchronize(&[Token::Let, Token::Next, Token::BraceClose]);
                }
            }
        }
        self.expect(&Token::BraceClose, "expected '}' to close node body");
    }

    // ── grid ─────────────────────────────────────────────────────────────

    fn parse_grid(&mut self) {
        self.advance();
        if !self.expect_ident("expected grid name after 'grid'") {
            return;
        }
        if !self.expect(&Token::Assign, "expected '=' after grid name") {
            return;
        }
        if !self.expect_ident("expected node name in grid definition") {
            return;
        }
        if !self.expect(&Token::BracketOpen, "expected '[' after node name in grid") {
            return;
        }
        loop {
            if matches!(self.peek(), Some(Token::IntLiteral(_))) {
                self.advance();
            } else {
                self.error("expected integer dimension in grid");
            }
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        if !self.expect(&Token::BracketClose, "expected ']' after grid dimensions") {
            return;
        }
        if !self.expect(&Token::ParenOpen, "expected '(' for grid arguments") {
            return;
        }
        if !self.check(&Token::ParenClose) {
            self.parse_args();
        }
        self.expect(&Token::ParenClose, "expected ')' after grid arguments");
        self.expect(&Token::Semicolon, "expected ';' after grid declaration");
    }

    // ── step ─────────────────────────────────────────────────────────────

    fn parse_step(&mut self) {
        self.advance();
        if !self.expect(&Token::BraceOpen, "expected '{' after 'step'") {
            return;
        }
        while !self.check(&Token::BraceClose) && !self.at_end() {
            match self.peek() {
                Some(Token::Run) => {
                    self.advance();
                    if !self.expect_ident("expected grid name after 'run'") {
                        break;
                    }
                    self.expect(&Token::Semicolon, "expected ';' after 'run'");
                }
                Some(Token::Next) => self.parse_next(),
                _ => {
                    self.error(&format!(
                        "unexpected '{}' in step body: expected 'run' or 'next'",
                        self.peek_tok()
                            .map(|t| t.lexeme.as_str())
                            .unwrap_or("<EOF>")
                    ));
                    self.advance();
                    self.synchronize(&[Token::Run, Token::Next, Token::BraceClose]);
                }
            }
        }
        self.expect(&Token::BraceClose, "expected '}' to close step body");
    }

    // ── statements ───────────────────────────────────────────────────────

    fn parse_statement(&mut self) {
        match self.peek() {
            Some(Token::Let) => self.parse_let(),
            Some(Token::Return) => self.parse_return(),
            Some(Token::Next) => self.parse_next(),
            _ => {
                self.parse_expression();
                self.expect(&Token::Semicolon, "expected ';' after expression");
            }
        }
    }

    fn parse_let(&mut self) {
        self.advance();
        if !self.expect_ident("expected variable name after 'let'") {
            return;
        }
        if !self.expect(&Token::Assign, "expected '=' after variable name") {
            return;
        }
        self.parse_expression();
        self.expect(&Token::Semicolon, "expected ';' after let statement");
    }

    fn parse_return(&mut self) {
        self.advance();
        self.parse_expression();
        self.expect(&Token::Semicolon, "expected ';' after return value");
    }

    fn parse_next(&mut self) {
        self.advance();
        if !self.expect_ident("expected target after 'next'") {
            return;
        }
        while matches!(self.peek(), Some(Token::BracketOpen) | Some(Token::Dot)) {
            if self.eat(&Token::BracketOpen) {
                self.parse_expression();
                self.expect(&Token::BracketClose, "expected ']' in index access");
            } else {
                self.advance();
                if !self.expect_ident("expected field name after '.'") {
                    return;
                }
            }
        }
        match self.peek() {
            Some(Token::Assign)
            | Some(Token::AddAssign)
            | Some(Token::SubAssign)
            | Some(Token::MulAssign)
            | Some(Token::DivAssign) => {
                self.advance();
            }
            _ => {
                self.error("expected assignment operator ('=', '+=', '-=', '*=', '/=') in next");
                return;
            }
        }
        self.parse_expression();
        self.expect(&Token::Semicolon, "expected ';' after next statement");
    }

    // ── expressions ────────────────────────────────────────────────────

    fn parse_expression(&mut self) {
        self.parse_ternary();
    }

    fn parse_ternary(&mut self) {
        self.parse_or();
        if self.eat(&Token::Question) {
            self.parse_expression();
            if !self.expect(&Token::Colon, "expected ':' in ternary expression") {
                return;
            }
            self.parse_expression();
        }
    }

    fn parse_or(&mut self) {
        self.parse_and();
        while self.eat(&Token::Or) {
            self.parse_and();
        }
    }

    fn parse_and(&mut self) {
        self.parse_comparison();
        while self.eat(&Token::And) {
            self.parse_comparison();
        }
    }

    fn parse_comparison(&mut self) {
        self.parse_addition();
        while let Some(Token::Eq) | Some(Token::Neq) | Some(Token::Lt) | Some(Token::Gt)
        | Some(Token::Le) | Some(Token::Ge) = self.peek()
        {
            self.advance();
            self.parse_addition();
        }
    }

    fn parse_addition(&mut self) {
        self.parse_multiplication();
        while matches!(self.peek(), Some(Token::Plus) | Some(Token::Minus)) {
            self.advance();
            self.parse_multiplication();
        }
    }

    fn parse_multiplication(&mut self) {
        self.parse_cast();
        while matches!(self.peek(), Some(Token::Star) | Some(Token::Slash)) {
            self.advance();
            self.parse_cast();
        }
    }

    fn parse_cast(&mut self) {
        self.parse_unary();
        while self.eat(&Token::As) {
            if !self.parse_type() {
                self.error("expected type after 'as'");
            }
        }
    }

    fn parse_unary(&mut self) {
        if matches!(self.peek(), Some(Token::Not) | Some(Token::Minus)) {
            self.advance();
        }
        self.parse_primary();
    }

    fn parse_primary(&mut self) {
        match self.peek() {
            Some(Token::IntLiteral(_))
            | Some(Token::FloatLiteral(_))
            | Some(Token::BoolTrue)
            | Some(Token::BoolFalse) => {
                self.advance();
            }
            Some(Token::Ident) => {
                self.advance();
            }
            Some(Token::BracketOpen) => {
                self.parse_array_literal();
            }
            Some(Token::ParenOpen) => {
                self.advance();
                self.parse_expression();
                self.expect(
                    &Token::ParenClose,
                    "expected ')' to close grouped expression",
                );
            }
            _ => {
                self.error(&format!(
                    "expected expression, found '{}'",
                    self.peek_tok()
                        .map(|t| t.lexeme.as_str())
                        .unwrap_or("<EOF>")
                ));
                return;
            }
        }

        // selector chain: .field  @0/@prev/@now  #[i,j]  [expr]  (args)  .method(args)
        loop {
            match self.peek() {
                Some(Token::Dot) => {
                    self.advance();
                    if self.check(&Token::Ident) {
                        self.advance();
                        if self.eat(&Token::ParenOpen) {
                            if !self.check(&Token::ParenClose) {
                                self.parse_args();
                            }
                            self.expect(&Token::ParenClose, "expected ')' after method arguments");
                        }
                    } else {
                        self.error("expected field or method name after '.'");
                        break;
                    }
                }
                Some(Token::At) => {
                    self.advance();
                    match self.peek() {
                        Some(Token::Now) | Some(Token::Prev) => {
                            self.advance();
                        }
                        Some(Token::IntLiteral(0)) => {
                            self.advance();
                        }
                        Some(Token::Minus) => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::IntLiteral(_))) {
                                self.advance();
                            } else {
                                self.error("expected integer after '-' in history access");
                            }
                        }
                        _ => {
                            self.error("expected '0', '-N', 'now', or 'prev' after '@'");
                            break;
                        }
                    }
                }
                Some(Token::Hash) => {
                    self.advance();
                    if !self.expect(&Token::BracketOpen, "expected '[' after '#'") {
                        break;
                    }
                    loop {
                        if matches!(self.peek(), Some(Token::IntLiteral(_))) {
                            self.advance();
                        } else {
                            self.error("expected integer offset in spatial access");
                        }
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&Token::BracketClose, "expected ']' after spatial offsets");
                }
                Some(Token::BracketOpen) => {
                    self.advance();
                    self.parse_expression();
                    self.expect(&Token::BracketClose, "expected ']' after index");
                }
                Some(Token::ParenOpen) => {
                    self.advance();
                    if !self.check(&Token::ParenClose) {
                        self.parse_args();
                    }
                    self.expect(&Token::ParenClose, "expected ')' after call arguments");
                }
                _ => break,
            }
        }
    }

    // ── вспомогательные ──────────────────────────────────────────────────

    fn parse_type(&mut self) -> bool {
        match self.peek() {
            Some(Token::Ident) => {
                self.advance();
                true
            }
            _ => {
                self.error(&format!(
                    "expected type, found '{}'",
                    self.peek_tok()
                        .map(|t| t.lexeme.as_str())
                        .unwrap_or("<EOF>")
                ));
                false
            }
        }
    }

    fn parse_bound(&mut self) -> bool {
        match self.peek() {
            Some(Token::Ident) => {
                let lex = self
                    .peek_tok()
                    .map(|t| t.lexeme.clone())
                    .unwrap_or_default();
                self.advance();
                if lex == "fixed" {
                    if !self.expect(&Token::ParenOpen, "expected '(' after 'fixed'") {
                        return false;
                    }
                    self.parse_literal_or_error("expected literal in fixed(...)");
                    if !self.expect(&Token::ParenClose, "expected ')' after fixed value") {
                        return false;
                    }
                } else if lex != "wrap" && lex != "clamp" {
                    self.error(&format!(
                        "expected boundary mode ('fixed', 'wrap', 'clamp'), found '{lex}'"
                    ));
                    return false;
                }
                true
            }
            _ => {
                self.error("expected boundary mode after '#'");
                false
            }
        }
    }

    fn parse_params(&mut self) {
        if self.check(&Token::ParenClose) || self.at_end() {
            return;
        }
        loop {
            if !self.expect_ident("expected parameter name") {
                break;
            }
            if !self.expect(&Token::Colon, "expected ':' after parameter name") {
                break;
            }
            if !self.parse_type() {
                break;
            }
            if !self.eat(&Token::Comma) {
                break;
            }
        }
    }

    fn parse_args(&mut self) {
        loop {
            self.parse_expression();
            if !self.eat(&Token::Comma) {
                break;
            }
        }
    }

    fn parse_array_literal(&mut self) {
        if !self.expect(&Token::BracketOpen, "expected '[' to start array literal") {
            return;
        }
        if self.check(&Token::BracketClose) {
            self.advance();
            return;
        }
        loop {
            if self.check(&Token::BracketOpen) {
                self.parse_array_literal();
            } else {
                self.parse_literal_or_error("expected literal in array");
            }
            if !self.eat(&Token::Comma) {
                break;
            }
            if self.check(&Token::BracketClose) {
                break;
            }
        }
        self.expect(&Token::BracketClose, "expected ']' to close array literal");
    }

    fn parse_literal_or_error(&mut self, msg: &str) {
        self.eat(&Token::Minus);
        match self.peek() {
            Some(Token::IntLiteral(_))
            | Some(Token::FloatLiteral(_))
            | Some(Token::BoolTrue)
            | Some(Token::BoolFalse) => {
                self.advance();
            }
            _ => self.error(msg),
        }
    }

    fn expect_ident(&mut self, msg: &str) -> bool {
        if self.check(&Token::Ident) {
            self.advance();
            true
        } else {
            self.error(msg);
            false
        }
    }
}
