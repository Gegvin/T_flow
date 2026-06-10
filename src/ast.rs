#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub declarations: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Struct(StructDecl),
    Const(ConstDecl),
    Table(TableDecl),
    Param(TableDecl),
    State(StateDecl),
    Fn(FnDecl),
    Node(NodeDecl),
    Grid(GridDecl),
    Step(StepDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub name: String,
    pub value: LiteralValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableDecl {
    pub name: String,
    pub bound: BoundaryMode,
    pub ty: TypeName,
    pub dimensions: Vec<i64>,
    pub values: LiteralValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateDecl {
    pub name: String,
    pub ty: TypeName,
    pub keep: i64,
    pub initial: LiteralValue,
    pub flat: LiteralValue,
    pub bound: BoundaryMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<ParamDecl>,
    pub return_type: TypeName,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeDecl {
    pub name: String,
    pub params: Vec<ParamDecl>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridDecl {
    pub name: String,
    pub node_name: String,
    pub dimensions: Vec<i64>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepDecl {
    pub body: Vec<StepStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDecl {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeName {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoundaryMode {
    Wrap,
    Clamp,
    Fixed(LiteralValue),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        expr: Expr,
    },
    Return {
        expr: Expr,
    },
    Next {
        target: Target,
        op: AssignOp,
        expr: Expr,
    },
    Expr {
        expr: Expr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepStmt {
    Run {
        name: String,
    },
    Next {
        target: Target,
        op: AssignOp,
        expr: Expr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub name: String,
    pub selectors: Vec<TargetSelector>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TargetSelector {
    Index(Expr),
    Field(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(LiteralValue),
    Ident(String),

    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },

    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    Cast {
        expr: Box<Expr>,
        ty: TypeName,
    },

    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },

    Selector {
        base: Box<Expr>,
        selector: ExprSelector,
    },

    Array(Vec<Expr>),

    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprSelector {
    Field(String),
    History(HistoryAccess),
    Spatial(Vec<i64>),
    Index(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HistoryAccess {
    Now,
    Prev,
    Offset(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<LiteralValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,

    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,

    And,
    Or,
}
