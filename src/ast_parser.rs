use crate::ast::*;
use crate::lexer_runner::LocatedToken;
use crate::token::Token;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

pub struct AstParser<'a> {
    tokens: &'a [LocatedToken<Token>],
    pos: usize,
    errors: Vec<ParseError>,
}

impl<'a> AstParser<'a> {
    pub fn new(tokens: &'a [LocatedToken<Token>]) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    pub fn parse_program(mut self) -> Result<Program, Vec<ParseError>> {
        let mut declarations = Vec::new();

        while !self.at_end() {
            match self.parse_declaration() {
                Some(decl) => declarations.push(decl),
                None => {
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

        if self.errors.is_empty() {
            Ok(Program { declarations })
        } else {
            Err(self.errors)
        }
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
        let (line, column) = self.current_pos();

        self.errors.push(ParseError {
            message: msg.to_string(),
            line,
            column,
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

    fn current_lexeme(&self) -> String {
        self.peek_tok()
            .map(|t| t.lexeme.clone())
            .unwrap_or_else(|| "<EOF>".to_string())
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

    fn take_ident(&mut self, msg: &str) -> Option<String> {
        if self.check(&Token::Ident) {
            let name = self
                .peek_tok()
                .map(|t| t.lexeme.clone())
                .unwrap_or_default();

            self.advance();

            Some(name)
        } else {
            self.error(msg);
            None
        }
    }

    fn take_int(&mut self, msg: &str) -> Option<i64> {
        match self.peek() {
            Some(Token::IntLiteral(value)) => {
                let value = *value;
                self.advance();
                Some(value)
            }
            _ => {
                self.error(msg);
                None
            }
        }
    }

    // ── верхний уровень ──────────────────────────────────────────────────

    fn parse_declaration(&mut self) -> Option<Decl> {
        match self.peek() {
            Some(Token::Struct) => self.parse_struct().map(Decl::Struct),
            Some(Token::Const) => self.parse_const().map(Decl::Const),
            Some(Token::Table) => self.parse_table_or_param(true).map(Decl::Table),
            Some(Token::Param) => self.parse_table_or_param(false).map(Decl::Param),
            Some(Token::State) => self.parse_state().map(Decl::State),
            Some(Token::Fn) => self.parse_fn().map(Decl::Fn),
            Some(Token::Node) => self.parse_node().map(Decl::Node),
            Some(Token::Grid) => self.parse_grid().map(Decl::Grid),
            Some(Token::Step) => self.parse_step().map(Decl::Step),
            _ => {
                self.error(&format!(
                    "unexpected token '{}': expected declaration",
                    self.current_lexeme()
                ));
                None
            }
        }
    }

    // ── struct ───────────────────────────────────────────────────────────

    fn parse_struct(&mut self) -> Option<StructDecl> {
        self.advance();

        let name = self.take_ident("expected struct name after 'struct'")?;

        if !self.expect(&Token::BraceOpen, "expected '{' after struct name") {
            return None;
        }

        let mut fields = Vec::new();

        while !self.check(&Token::BraceClose) && !self.at_end() {
            let field_name = self.take_ident("expected field name")?;

            if !self.expect(&Token::Colon, "expected ':' after field name") {
                return None;
            }

            let ty = self.parse_type()?;

            fields.push(FieldDecl {
                name: field_name,
                ty,
            });

            self.eat(&Token::Comma);
        }

        self.expect(&Token::BraceClose, "expected '}' to close struct");

        Some(StructDecl { name, fields })
    }

    // ── const ────────────────────────────────────────────────────────────

    fn parse_const(&mut self) -> Option<ConstDecl> {
        self.advance();

        let name = self.take_ident("expected constant name after 'const'")?;

        if !self.expect(&Token::Assign, "expected '=' after constant name") {
            return None;
        }

        let value = self.parse_literal_value("expected literal value in const declaration")?;

        self.expect(&Token::Semicolon, "expected ';' after const declaration");

        Some(ConstDecl { name, value })
    }

    // ── table / param ────────────────────────────────────────────────────

    fn parse_table_or_param(&mut self, is_table: bool) -> Option<TableDecl> {
        self.advance();

        let kw = if is_table { "table" } else { "param" };

        let name = self.take_ident(&format!("expected name after '{kw}'"))?;

        if !self.expect(&Token::Hash, &format!("expected '#' after {kw} name")) {
            return None;
        }

        let bound = self.parse_bound()?;

        if !self.expect(&Token::Colon, "expected ':' before type") {
            return None;
        }

        let ty = self.parse_type()?;

        let mut dimensions = Vec::new();

        while self.check(&Token::BracketOpen) {
            self.advance();

            let size = self.take_int("expected integer size in dimension")?;
            dimensions.push(size);

            if !self.expect(&Token::BracketClose, "expected ']' after dimension size") {
                return None;
            }
        }

        if !self.expect(&Token::Assign, "expected '=' before array literal") {
            return None;
        }

        let values = self.parse_array_literal_value()?;

        self.expect(
            &Token::Semicolon,
            &format!("expected ';' after {kw} declaration"),
        );

        Some(TableDecl {
            name,
            bound,
            ty,
            dimensions,
            values,
        })
    }

    // ── state ────────────────────────────────────────────────────────────

    fn parse_state(&mut self) -> Option<StateDecl> {
        self.advance();

        let name = self.take_ident("expected state name")?;

        if !self.expect(&Token::Colon, "expected ':' after state name") {
            return None;
        }

        let ty = self.parse_type()?;

        if !self.expect(&Token::Keep, "expected 'keep' after type in state") {
            return None;
        }

        if !self.expect(&Token::ParenOpen, "expected '(' after 'keep'") {
            return None;
        }

        let keep = self.take_int("expected integer history depth in keep(...)")?;

        if !self.expect(&Token::ParenClose, "expected ')' after history depth") {
            return None;
        }

        if !self.expect(&Token::Assign, "expected '=' after keep(...)") {
            return None;
        }

        let initial = if self.check(&Token::BracketOpen) {
            self.parse_array_literal_value()?
        } else {
            self.parse_literal_value("expected literal or array as initial value")?
        };

        if !self.expect(&Token::At, "expected '@' after initial values") {
            return None;
        }

        let flat = self.parse_array_literal_value()?;

        if !self.expect(&Token::Hash, "expected '#' after flat array") {
            return None;
        }

        let bound = self.parse_bound()?;

        self.expect(&Token::Semicolon, "expected ';' after state declaration");

        Some(StateDecl {
            name,
            ty,
            keep,
            initial,
            flat,
            bound,
        })
    }

    // ── fn ───────────────────────────────────────────────────────────────

    fn parse_fn(&mut self) -> Option<FnDecl> {
        self.advance();

        let name = self.take_ident("expected function name after 'fn'")?;

        if !self.expect(&Token::ParenOpen, "expected '(' after function name") {
            return None;
        }

        let params = self.parse_params();

        if !self.expect(&Token::ParenClose, "expected ')' after parameters") {
            return None;
        }

        if !self.expect(&Token::Minus, "expected '->' after ')'") {
            return None;
        }

        if !self.expect(&Token::Gt, "expected '>' in '->'") {
            return None;
        }

        let return_type = self.parse_type()?;

        if !self.expect(&Token::BraceOpen, "expected '{' to open function body") {
            return None;
        }

        let mut body = Vec::new();

        while !self.check(&Token::BraceClose) && !self.at_end() {
            match self.parse_statement() {
                Some(stmt) => body.push(stmt),
                None => {
                    self.advance();
                    self.synchronize(&[
                        Token::Let,
                        Token::Return,
                        Token::Next,
                        Token::BraceClose,
                        Token::Semicolon,
                    ]);
                    self.eat(&Token::Semicolon);
                }
            }
        }

        self.expect(&Token::BraceClose, "expected '}' to close function body");

        Some(FnDecl {
            name,
            params,
            return_type,
            body,
        })
    }

    // ── node ─────────────────────────────────────────────────────────────

    fn parse_node(&mut self) -> Option<NodeDecl> {
        self.advance();

        let name = self.take_ident("expected node name after 'node'")?;

        if !self.expect(&Token::ParenOpen, "expected '(' after node name") {
            return None;
        }

        let params = self.parse_params();

        if !self.expect(&Token::ParenClose, "expected ')' after parameters") {
            return None;
        }

        if !self.expect(&Token::BraceOpen, "expected '{' to open node body") {
            return None;
        }

        let mut body = Vec::new();

        while !self.check(&Token::BraceClose) && !self.at_end() {
            match self.peek() {
                Some(Token::Let) | Some(Token::Next) => {
                    if let Some(stmt) = self.parse_statement() {
                        body.push(stmt);
                    }
                }
                _ => {
                    self.error(&format!(
                        "unexpected '{}' in node body: only 'let' and 'next' are allowed",
                        self.current_lexeme()
                    ));
                    self.advance();
                    self.synchronize(&[Token::Let, Token::Next, Token::BraceClose]);
                }
            }
        }

        self.expect(&Token::BraceClose, "expected '}' to close node body");

        Some(NodeDecl { name, params, body })
    }

    // ── grid ─────────────────────────────────────────────────────────────

    fn parse_grid(&mut self) -> Option<GridDecl> {
        self.advance();

        let name = self.take_ident("expected grid name after 'grid'")?;

        if !self.expect(&Token::Assign, "expected '=' after grid name") {
            return None;
        }

        let node_name = self.take_ident("expected node name in grid definition")?;

        if !self.expect(&Token::BracketOpen, "expected '[' after node name in grid") {
            return None;
        }

        let mut dimensions = Vec::new();

        loop {
            let dim = self.take_int("expected integer dimension in grid")?;
            dimensions.push(dim);

            if !self.eat(&Token::Comma) {
                break;
            }
        }

        if !self.expect(&Token::BracketClose, "expected ']' after grid dimensions") {
            return None;
        }

        if !self.expect(&Token::ParenOpen, "expected '(' for grid arguments") {
            return None;
        }

        let args = if self.check(&Token::ParenClose) {
            Vec::new()
        } else {
            self.parse_args()
        };

        self.expect(&Token::ParenClose, "expected ')' after grid arguments");
        self.expect(&Token::Semicolon, "expected ';' after grid declaration");

        Some(GridDecl {
            name,
            node_name,
            dimensions,
            args,
        })
    }

    // ── step ─────────────────────────────────────────────────────────────

    fn parse_step(&mut self) -> Option<StepDecl> {
        self.advance();

        if !self.expect(&Token::BraceOpen, "expected '{' after 'step'") {
            return None;
        }

        let mut body = Vec::new();

        while !self.check(&Token::BraceClose) && !self.at_end() {
            match self.peek() {
                Some(Token::Run) => {
                    self.advance();

                    let name = self.take_ident("expected grid name after 'run'")?;

                    self.expect(&Token::Semicolon, "expected ';' after 'run'");

                    body.push(StepStmt::Run { name });
                }
                Some(Token::Next) => {
                    if let Some(Stmt::Next { target, op, expr }) = self.parse_next() {
                        body.push(StepStmt::Next { target, op, expr });
                    }
                }
                _ => {
                    self.error(&format!(
                        "unexpected '{}' in step body: expected 'run' or 'next'",
                        self.current_lexeme()
                    ));
                    self.advance();
                    self.synchronize(&[Token::Run, Token::Next, Token::BraceClose]);
                }
            }
        }

        self.expect(&Token::BraceClose, "expected '}' to close step body");

        Some(StepDecl { body })
    }

    // ── инструкции ───────────────────────────────────────────────────────

    fn parse_statement(&mut self) -> Option<Stmt> {
        match self.peek() {
            Some(Token::Let) => self.parse_let(),
            Some(Token::Return) => self.parse_return(),
            Some(Token::Next) => self.parse_next(),
            _ => {
                let expr = self.parse_expression();

                self.expect(&Token::Semicolon, "expected ';' after expression");

                Some(Stmt::Expr { expr })
            }
        }
    }

    fn parse_let(&mut self) -> Option<Stmt> {
        self.advance();

        let name = self.take_ident("expected variable name after 'let'")?;

        if !self.expect(&Token::Assign, "expected '=' after variable name") {
            return None;
        }

        let expr = self.parse_expression();

        self.expect(&Token::Semicolon, "expected ';' after let statement");

        Some(Stmt::Let { name, expr })
    }

    fn parse_return(&mut self) -> Option<Stmt> {
        self.advance();

        let expr = self.parse_expression();

        self.expect(&Token::Semicolon, "expected ';' after return value");

        Some(Stmt::Return { expr })
    }

    fn parse_next(&mut self) -> Option<Stmt> {
        self.advance();

        let target = self.parse_target()?;

        let op = match self.peek() {
            Some(Token::Assign) => AssignOp::Assign,
            Some(Token::AddAssign) => AssignOp::AddAssign,
            Some(Token::SubAssign) => AssignOp::SubAssign,
            Some(Token::MulAssign) => AssignOp::MulAssign,
            Some(Token::DivAssign) => AssignOp::DivAssign,
            _ => {
                self.error("expected assignment operator ('=', '+=', '-=', '*=', '/=') in next");
                return None;
            }
        };

        self.advance();

        let expr = self.parse_expression();

        self.expect(&Token::Semicolon, "expected ';' after next statement");

        Some(Stmt::Next { target, op, expr })
    }

    fn parse_target(&mut self) -> Option<Target> {
        let name = self.take_ident("expected target after 'next'")?;
        let mut selectors = Vec::new();

        loop {
            match self.peek() {
                Some(Token::BracketOpen) => {
                    self.advance();

                    let index = self.parse_expression();

                    self.expect(&Token::BracketClose, "expected ']' in index access");

                    selectors.push(TargetSelector::Index(index));
                }
                Some(Token::Dot) => {
                    self.advance();

                    let field = self.take_ident("expected field name after '.'")?;

                    selectors.push(TargetSelector::Field(field));
                }
                _ => break,
            }
        }

        Some(Target { name, selectors })
    }

    // ── выражения ────────────────────────────────────────────────────────

    fn parse_expression(&mut self) -> Expr {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Expr {
        let cond = self.parse_or();

        if self.eat(&Token::Question) {
            let then_expr = self.parse_expression();

            if !self.expect(&Token::Colon, "expected ':' in ternary expression") {
                return Expr::Error;
            }

            let else_expr = self.parse_expression();

            Expr::Ternary {
                cond: Box::new(cond),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            }
        } else {
            cond
        }
    }

    fn parse_or(&mut self) -> Expr {
        let mut expr = self.parse_and();

        while self.eat(&Token::Or) {
            let right = self.parse_and();

            expr = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_and(&mut self) -> Expr {
        let mut expr = self.parse_comparison();

        while self.eat(&Token::And) {
            let right = self.parse_comparison();

            expr = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut expr = self.parse_addition();

        loop {
            let op = match self.peek() {
                Some(Token::Eq) => BinaryOp::Eq,
                Some(Token::Neq) => BinaryOp::Neq,
                Some(Token::Lt) => BinaryOp::Lt,
                Some(Token::Gt) => BinaryOp::Gt,
                Some(Token::Le) => BinaryOp::Le,
                Some(Token::Ge) => BinaryOp::Ge,
                _ => break,
            };

            self.advance();

            let right = self.parse_addition();

            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_addition(&mut self) -> Expr {
        let mut expr = self.parse_multiplication();

        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinaryOp::Add,
                Some(Token::Minus) => BinaryOp::Sub,
                _ => break,
            };

            self.advance();

            let right = self.parse_multiplication();

            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_multiplication(&mut self) -> Expr {
        let mut expr = self.parse_cast();

        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinaryOp::Mul,
                Some(Token::Slash) => BinaryOp::Div,
                _ => break,
            };

            self.advance();

            let right = self.parse_cast();

            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_cast(&mut self) -> Expr {
        let mut expr = self.parse_unary();

        while self.eat(&Token::As) {
            let ty = match self.parse_type() {
                Some(ty) => ty,
                None => {
                    self.error("expected type after 'as'");
                    return Expr::Error;
                }
            };

            expr = Expr::Cast {
                expr: Box::new(expr),
                ty,
            };
        }

        expr
    }

    fn parse_unary(&mut self) -> Expr {
        match self.peek() {
            Some(Token::Not) => {
                self.advance();

                let expr = self.parse_unary();

                Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                }
            }
            Some(Token::Minus) => {
                self.advance();

                let expr = self.parse_unary();

                Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                }
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Expr {
        let mut expr = match self.peek() {
            Some(Token::IntLiteral(value)) => {
                let value = *value;
                self.advance();

                Expr::Literal(LiteralValue::Int(value))
            }
            Some(Token::FloatLiteral(value)) => {
                let value = *value;
                self.advance();

                Expr::Literal(LiteralValue::Float(value))
            }
            Some(Token::BoolTrue) => {
                self.advance();

                Expr::Literal(LiteralValue::Bool(true))
            }
            Some(Token::BoolFalse) => {
                self.advance();

                Expr::Literal(LiteralValue::Bool(false))
            }
            Some(Token::Ident) => {
                let name = self
                    .peek_tok()
                    .map(|t| t.lexeme.clone())
                    .unwrap_or_default();

                self.advance();

                Expr::Ident(name)
            }
            Some(Token::BracketOpen) => self.parse_array_expr(),
            Some(Token::ParenOpen) => {
                self.advance();

                let expr = self.parse_expression();

                self.expect(
                    &Token::ParenClose,
                    "expected ')' to close grouped expression",
                );

                expr
            }
            _ => {
                self.error(&format!(
                    "expected expression, found '{}'",
                    self.current_lexeme()
                ));

                return Expr::Error;
            }
        };

        loop {
            match self.peek() {
                Some(Token::Dot) => {
                    self.advance();

                    let field = match self.take_ident("expected field or method name after '.'") {
                        Some(field) => field,
                        None => return Expr::Error,
                    };

                    expr = Expr::Selector {
                        base: Box::new(expr),
                        selector: ExprSelector::Field(field),
                    };

                    if self.eat(&Token::ParenOpen) {
                        let args = if self.check(&Token::ParenClose) {
                            Vec::new()
                        } else {
                            self.parse_args()
                        };

                        self.expect(&Token::ParenClose, "expected ')' after method arguments");

                        expr = Expr::Call {
                            callee: Box::new(expr),
                            args,
                        };
                    }
                }

                Some(Token::At) => {
                    self.advance();

                    let access = match self.peek() {
                        Some(Token::Now) => {
                            self.advance();
                            HistoryAccess::Now
                        }
                        Some(Token::Prev) => {
                            self.advance();
                            HistoryAccess::Prev
                        }
                        Some(Token::IntLiteral(value)) => {
                            let value = *value;
                            self.advance();
                            HistoryAccess::Offset(value)
                        }
                        Some(Token::Minus) => {
                            self.advance();

                            let value = match self
                                .take_int("expected integer after '-' in history access")
                            {
                                Some(value) => value,
                                None => return Expr::Error,
                            };

                            HistoryAccess::Offset(-value)
                        }
                        _ => {
                            self.error("expected '0', '-N', 'now', or 'prev' after '@'");
                            return Expr::Error;
                        }
                    };

                    expr = Expr::Selector {
                        base: Box::new(expr),
                        selector: ExprSelector::History(access),
                    };
                }

                Some(Token::Hash) => {
                    self.advance();

                    if !self.expect(&Token::BracketOpen, "expected '[' after '#'") {
                        return Expr::Error;
                    }

                    let mut offsets = Vec::new();

                    loop {
                        let sign = if self.eat(&Token::Minus) { -1 } else { 1 };

                        let value = match self.take_int("expected integer offset in spatial access")
                        {
                            Some(value) => value,
                            None => return Expr::Error,
                        };

                        offsets.push(sign * value);

                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }

                    self.expect(&Token::BracketClose, "expected ']' after spatial offsets");

                    expr = Expr::Selector {
                        base: Box::new(expr),
                        selector: ExprSelector::Spatial(offsets),
                    };
                }

                Some(Token::BracketOpen) => {
                    self.advance();

                    let index = self.parse_expression();

                    self.expect(&Token::BracketClose, "expected ']' after index");

                    expr = Expr::Selector {
                        base: Box::new(expr),
                        selector: ExprSelector::Index(Box::new(index)),
                    };
                }

                Some(Token::ParenOpen) => {
                    self.advance();

                    let args = if self.check(&Token::ParenClose) {
                        Vec::new()
                    } else {
                        self.parse_args()
                    };

                    self.expect(&Token::ParenClose, "expected ')' after call arguments");

                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                    };
                }

                _ => break,
            }
        }

        expr
    }

    fn parse_array_expr(&mut self) -> Expr {
        self.advance();

        let mut values = Vec::new();

        if self.check(&Token::BracketClose) {
            self.advance();
            return Expr::Array(values);
        }

        loop {
            let value = self.parse_expression();
            values.push(value);

            if !self.eat(&Token::Comma) {
                break;
            }

            if self.check(&Token::BracketClose) {
                break;
            }
        }

        self.expect(&Token::BracketClose, "expected ']' to close array literal");

        Expr::Array(values)
    }

    // ── вспомогательные функции ──────────────────────────────────────────

    fn parse_type(&mut self) -> Option<TypeName> {
        let name = self.take_ident("expected type")?;

        Some(TypeName { name })
    }

    fn parse_bound(&mut self) -> Option<BoundaryMode> {
        let name = self.take_ident("expected boundary mode after '#'")?;

        match name.as_str() {
            "wrap" => Some(BoundaryMode::Wrap),
            "clamp" => Some(BoundaryMode::Clamp),
            "fixed" => {
                if !self.expect(&Token::ParenOpen, "expected '(' after 'fixed'") {
                    return None;
                }

                let value = self.parse_literal_value("expected literal in fixed(...)")?;

                if !self.expect(&Token::ParenClose, "expected ')' after fixed value") {
                    return None;
                }

                Some(BoundaryMode::Fixed(value))
            }
            _ => {
                self.error(&format!(
                    "expected boundary mode ('fixed', 'wrap', 'clamp'), found '{name}'"
                ));

                None
            }
        }
    }

    fn parse_params(&mut self) -> Vec<ParamDecl> {
        let mut params = Vec::new();

        if self.check(&Token::ParenClose) || self.at_end() {
            return params;
        }

        while let Some(name) = self.take_ident("expected parameter name") {
            if !self.expect(&Token::Colon, "expected ':' after parameter name") {
                break;
            }

            let ty = match self.parse_type() {
                Some(ty) => ty,
                None => break,
            };

            params.push(ParamDecl { name, ty });

            if !self.eat(&Token::Comma) {
                break;
            }
        }

        params
    }

    fn parse_args(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();

        loop {
            args.push(self.parse_expression());

            if !self.eat(&Token::Comma) {
                break;
            }
        }

        args
    }

    fn parse_array_literal_value(&mut self) -> Option<LiteralValue> {
        if !self.expect(&Token::BracketOpen, "expected '[' to start array literal") {
            return None;
        }

        let mut values = Vec::new();

        if self.check(&Token::BracketClose) {
            self.advance();
            return Some(LiteralValue::Array(values));
        }

        loop {
            let value = if self.check(&Token::BracketOpen) {
                self.parse_array_literal_value()?
            } else {
                self.parse_literal_value("expected literal in array")?
            };

            values.push(value);

            if !self.eat(&Token::Comma) {
                break;
            }

            if self.check(&Token::BracketClose) {
                break;
            }
        }

        self.expect(&Token::BracketClose, "expected ']' to close array literal");

        Some(LiteralValue::Array(values))
    }

    fn parse_literal_value(&mut self, msg: &str) -> Option<LiteralValue> {
        let negative = self.eat(&Token::Minus);

        match self.peek() {
            Some(Token::IntLiteral(value)) => {
                let mut value = *value;

                if negative {
                    value = -value;
                }

                self.advance();

                Some(LiteralValue::Int(value))
            }
            Some(Token::FloatLiteral(value)) => {
                let mut value = *value;

                if negative {
                    value = -value;
                }

                self.advance();

                Some(LiteralValue::Float(value))
            }
            Some(Token::BoolTrue) => {
                if negative {
                    self.error("boolean literal cannot be negative");
                    return None;
                }

                self.advance();

                Some(LiteralValue::Bool(true))
            }
            Some(Token::BoolFalse) => {
                if negative {
                    self.error("boolean literal cannot be negative");
                    return None;
                }

                self.advance();

                Some(LiteralValue::Bool(false))
            }
            _ => {
                self.error(msg);
                None
            }
        }
    }
}
