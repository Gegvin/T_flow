use crate::ast::*;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
}

impl RuntimeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
}

#[derive(Debug, Clone)]
struct StateRuntime {
    values: Vec<Value>,
    next_values: Vec<Value>,
    bound: BoundaryMode,
}

pub struct Interpreter {
    consts: BTreeMap<String, Value>,
    states: BTreeMap<String, StateRuntime>,
    nodes: BTreeMap<String, NodeDecl>,
    grids: BTreeMap<String, GridDecl>,
    steps: Vec<StepDecl>,
}

impl Interpreter {
    pub fn new(program: Program) -> Result<Self, RuntimeError> {
        let mut interpreter = Self {
            consts: BTreeMap::new(),
            states: BTreeMap::new(),
            nodes: BTreeMap::new(),
            grids: BTreeMap::new(),
            steps: Vec::new(),
        };

        for decl in program.declarations {
            match decl {
                Decl::Const(decl) => {
                    let value = literal_to_value(&decl.value);
                    interpreter.consts.insert(decl.name, value);
                }
                Decl::State(decl) => {
                    let values = match literal_to_value(&decl.flat) {
                        Value::Array(values) => values,
                        other => vec![other],
                    };

                    interpreter.states.insert(
                        decl.name,
                        StateRuntime {
                            next_values: values.clone(),
                            values,
                            bound: decl.bound,
                        },
                    );
                }
                Decl::Node(decl) => {
                    interpreter.nodes.insert(decl.name.clone(), decl);
                }
                Decl::Grid(decl) => {
                    interpreter.grids.insert(decl.name.clone(), decl);
                }
                Decl::Step(decl) => {
                    interpreter.steps.push(decl);
                }
                _ => {}
            }
        }

        if interpreter.steps.is_empty() {
            return Err(RuntimeError::new("program does not contain step block"));
        }

        Ok(interpreter)
    }

    pub fn run_steps(&mut self, count: usize) -> Result<String, RuntimeError> {
        let mut output = String::new();

        output.push_str(&self.format_states(0));

        for step_number in 1..=count {
            self.run_one_step()?;
            output.push_str(&self.format_states(step_number));
        }

        Ok(output)
    }

    fn run_one_step(&mut self) -> Result<(), RuntimeError> {
        for state in self.states.values_mut() {
            state.next_values = state.values.clone();
        }

        let steps = self.steps.clone();

        for step in steps {
            for stmt in step.body {
                match stmt {
                    StepStmt::Run { name } => {
                        self.run_grid(&name)?;
                    }
                    StepStmt::Next { target, op, expr } => {
                        let env = HashMap::new();
                        let value = self.eval_expr(&expr, &env, 0)?;
                        self.assign_next(&target, op, value, 0)?;
                    }
                }
            }
        }

        for state in self.states.values_mut() {
            state.values = state.next_values.clone();
        }

        Ok(())
    }

    fn run_grid(&mut self, grid_name: &str) -> Result<(), RuntimeError> {
        let grid = self
            .grids
            .get(grid_name)
            .cloned()
            .ok_or_else(|| RuntimeError::new(format!("unknown grid '{grid_name}'")))?;

        let node = self
            .nodes
            .get(&grid.node_name)
            .cloned()
            .ok_or_else(|| RuntimeError::new(format!("unknown node '{}'", grid.node_name)))?;

        let size = grid
            .dimensions
            .first()
            .copied()
            .ok_or_else(|| RuntimeError::new("grid must have at least one dimension"))?;

        if size < 0 {
            return Err(RuntimeError::new("grid size cannot be negative"));
        }

        let args = self.eval_grid_args(&grid.args)?;

        if args.len() != node.params.len() {
            return Err(RuntimeError::new(format!(
                "node '{}' expects {} args, got {}",
                node.name,
                node.params.len(),
                args.len()
            )));
        }

        for cell_index in 0..size as usize {
            let mut env = HashMap::new();

            for (param, value) in node.params.iter().zip(args.iter()) {
                env.insert(param.name.clone(), value.clone());
            }

            for stmt in &node.body {
                self.execute_stmt(stmt, &mut env, cell_index)?;
            }
        }

        Ok(())
    }

    fn eval_grid_args(&self, args: &[Expr]) -> Result<Vec<Value>, RuntimeError> {
        let env = HashMap::new();
        let mut result = Vec::new();

        for arg in args {
            result.push(self.eval_expr(arg, &env, 0)?);
        }

        Ok(result)
    }

    fn execute_stmt(
        &mut self,
        stmt: &Stmt,
        env: &mut HashMap<String, Value>,
        cell_index: usize,
    ) -> Result<Option<Value>, RuntimeError> {
        match stmt {
            Stmt::Let { name, expr } => {
                let value = self.eval_expr(expr, env, cell_index)?;
                env.insert(name.clone(), value);
                Ok(None)
            }
            Stmt::Return { expr } => {
                let value = self.eval_expr(expr, env, cell_index)?;
                Ok(Some(value))
            }
            Stmt::Next { target, op, expr } => {
                let value = self.eval_expr(expr, env, cell_index)?;
                self.assign_next(target, op.clone(), value, cell_index)?;
                Ok(None)
            }
            Stmt::Expr { expr } => {
                self.eval_expr(expr, env, cell_index)?;
                Ok(None)
            }
        }
    }

    fn assign_next(
        &mut self,
        target: &Target,
        op: AssignOp,
        value: Value,
        cell_index: usize,
    ) -> Result<(), RuntimeError> {
        if !target.selectors.is_empty() {
            return Err(RuntimeError::new(
                "next with field/index selectors is not supported yet",
            ));
        }

        let state = self
            .states
            .get_mut(&target.name)
            .ok_or_else(|| RuntimeError::new(format!("unknown state '{}'", target.name)))?;

        if cell_index >= state.next_values.len() {
            return Err(RuntimeError::new(format!(
                "cell index {} is out of bounds for state '{}'",
                cell_index, target.name
            )));
        }

        let old_value = state.next_values[cell_index].clone();

        let new_value = match op {
            AssignOp::Assign => value,
            AssignOp::AddAssign => apply_binary(BinaryOp::Add, old_value, value)?,
            AssignOp::SubAssign => apply_binary(BinaryOp::Sub, old_value, value)?,
            AssignOp::MulAssign => apply_binary(BinaryOp::Mul, old_value, value)?,
            AssignOp::DivAssign => apply_binary(BinaryOp::Div, old_value, value)?,
        };

        state.next_values[cell_index] = new_value;

        Ok(())
    }

    fn eval_expr(
        &self,
        expr: &Expr,
        env: &HashMap<String, Value>,
        cell_index: usize,
    ) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Literal(value) => Ok(literal_to_value(value)),

            Expr::Ident(name) => {
                if let Some(value) = env.get(name) {
                    return Ok(value.clone());
                }

                if let Some(value) = self.consts.get(name) {
                    return Ok(value.clone());
                }

                if self.states.contains_key(name) {
                    return self.read_state(name, cell_index as i64);
                }

                Err(RuntimeError::new(format!("unknown identifier '{name}'")))
            }

            Expr::Unary { op, expr } => {
                let value = self.eval_expr(expr, env, cell_index)?;

                match op {
                    UnaryOp::Neg => match value {
                        Value::Int(value) => Ok(Value::Int(-value)),
                        Value::Float(value) => Ok(Value::Float(-value)),
                        _ => Err(RuntimeError::new("unary '-' expects number")),
                    },
                    UnaryOp::Not => match value {
                        Value::Bool(value) => Ok(Value::Bool(!value)),
                        _ => Err(RuntimeError::new("unary '!' expects bool")),
                    },
                }
            }

            Expr::Binary { op, left, right } => {
                let left = self.eval_expr(left, env, cell_index)?;
                let right = self.eval_expr(right, env, cell_index)?;

                apply_binary(op.clone(), left, right)
            }

            Expr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let cond = self.eval_expr(cond, env, cell_index)?;

                match cond {
                    Value::Bool(true) => self.eval_expr(then_expr, env, cell_index),
                    Value::Bool(false) => self.eval_expr(else_expr, env, cell_index),
                    _ => Err(RuntimeError::new("ternary condition must be bool")),
                }
            }

            Expr::Cast { expr, ty } => {
                let value = self.eval_expr(expr, env, cell_index)?;
                cast_value(value, &ty.name)
            }

            Expr::Selector { base, selector } => match selector {
                ExprSelector::Spatial(offsets) => {
                    let name = match base.as_ref() {
                        Expr::Ident(name) => name,
                        _ => {
                            return Err(RuntimeError::new(
                                "spatial access is supported only for state name",
                            ));
                        }
                    };

                    let offset = offsets.first().copied().unwrap_or(0);
                    self.read_state(name, cell_index as i64 + offset)
                }

                ExprSelector::Index(index_expr) => {
                    let base_value = self.eval_expr(base, env, cell_index)?;
                    let index_value = self.eval_expr(index_expr, env, cell_index)?;

                    let index = match index_value {
                        Value::Int(value) => value,
                        _ => return Err(RuntimeError::new("array index must be Int")),
                    };

                    match base_value {
                        Value::Array(values) => values
                            .get(index as usize)
                            .cloned()
                            .ok_or_else(|| RuntimeError::new("array index out of bounds")),
                        _ => Err(RuntimeError::new("index access expects array")),
                    }
                }

                _ => Err(RuntimeError::new("this selector is not supported yet")),
            },

            Expr::Array(values) => {
                let mut result = Vec::new();

                for value in values {
                    result.push(self.eval_expr(value, env, cell_index)?);
                }

                Ok(Value::Array(result))
            }

            Expr::Call { .. } => Err(RuntimeError::new("function calls are not supported yet")),

            Expr::Error => Err(RuntimeError::new("cannot evaluate invalid expression")),
        }
    }

    fn read_state(&self, name: &str, index: i64) -> Result<Value, RuntimeError> {
        let state = self
            .states
            .get(name)
            .ok_or_else(|| RuntimeError::new(format!("unknown state '{name}'")))?;

        let len = state.values.len() as i64;

        if len == 0 {
            return Err(RuntimeError::new(format!("state '{name}' is empty")));
        }

        let real_index = match &state.bound {
            BoundaryMode::Wrap => {
                let mut value = index % len;

                if value < 0 {
                    value += len;
                }

                value
            }
            BoundaryMode::Clamp => {
                if index < 0 {
                    0
                } else if index >= len {
                    len - 1
                } else {
                    index
                }
            }
            BoundaryMode::Fixed(value) => {
                if index < 0 || index >= len {
                    return Ok(literal_to_value(value));
                }

                index
            }
        };

        Ok(state.values[real_index as usize].clone())
    }

    fn format_states(&self, step: usize) -> String {
        let mut output = String::new();

        output.push_str(&format!("step {step}\n"));

        for (name, state) in &self.states {
            output.push_str(&format!("{name} = {}\n", format_values(&state.values)));
        }

        output
    }
}

fn literal_to_value(value: &LiteralValue) -> Value {
    match value {
        LiteralValue::Int(value) => Value::Int(*value),
        LiteralValue::Float(value) => Value::Float(*value),
        LiteralValue::Bool(value) => Value::Bool(*value),
        LiteralValue::Array(values) => Value::Array(values.iter().map(literal_to_value).collect()),
    }
}

fn apply_binary(op: BinaryOp, left: Value, right: Value) -> Result<Value, RuntimeError> {
    match op {
        BinaryOp::Add => numeric_op(left, right, |a, b| a + b, |a, b| a + b),
        BinaryOp::Sub => numeric_op(left, right, |a, b| a - b, |a, b| a - b),
        BinaryOp::Mul => numeric_op(left, right, |a, b| a * b, |a, b| a * b),
        BinaryOp::Div => numeric_op(left, right, |a, b| a / b, |a, b| a / b),

        BinaryOp::Eq => Ok(Value::Bool(left == right)),
        BinaryOp::Neq => Ok(Value::Bool(left != right)),

        BinaryOp::Lt => compare_op(left, right, |a, b| a < b, |a, b| a < b),
        BinaryOp::Gt => compare_op(left, right, |a, b| a > b, |a, b| a > b),
        BinaryOp::Le => compare_op(left, right, |a, b| a <= b, |a, b| a <= b),
        BinaryOp::Ge => compare_op(left, right, |a, b| a >= b, |a, b| a >= b),

        BinaryOp::And => match (left, right) {
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a && b)),
            _ => Err(RuntimeError::new("'&&' expects bool values")),
        },

        BinaryOp::Or => match (left, right) {
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a || b)),
            _ => Err(RuntimeError::new("'||' expects bool values")),
        },
    }
}

fn numeric_op(
    left: Value,
    right: Value,
    int_op: fn(i64, i64) -> i64,
    float_op: fn(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(a, b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(a, b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(float_op(a as f64, b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(float_op(a, b as f64))),
        _ => Err(RuntimeError::new("numeric operation expects numbers")),
    }
}

fn compare_op(
    left: Value,
    right: Value,
    int_op: fn(i64, i64) -> bool,
    float_op: fn(f64, f64) -> bool,
) -> Result<Value, RuntimeError> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(int_op(a, b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(float_op(a, b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Bool(float_op(a as f64, b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(float_op(a, b as f64))),
        _ => Err(RuntimeError::new("comparison expects numbers")),
    }
}

fn cast_value(value: Value, ty: &str) -> Result<Value, RuntimeError> {
    match ty {
        "Int" => match value {
            Value::Int(value) => Ok(Value::Int(value)),
            Value::Float(value) => Ok(Value::Int(value as i64)),
            Value::Bool(value) => Ok(Value::Int(if value { 1 } else { 0 })),
            _ => Err(RuntimeError::new("cannot cast value to Int")),
        },
        "Float" => match value {
            Value::Int(value) => Ok(Value::Float(value as f64)),
            Value::Float(value) => Ok(Value::Float(value)),
            Value::Bool(value) => Ok(Value::Float(if value { 1.0 } else { 0.0 })),
            _ => Err(RuntimeError::new("cannot cast value to Float")),
        },
        "Bool" => match value {
            Value::Bool(value) => Ok(Value::Bool(value)),
            Value::Int(value) => Ok(Value::Bool(value != 0)),
            Value::Float(value) => Ok(Value::Bool(value != 0.0)),
            _ => Err(RuntimeError::new("cannot cast value to Bool")),
        },
        _ => Err(RuntimeError::new(format!("unknown type '{ty}'"))),
    }
}

fn format_values(values: &[Value]) -> String {
    let parts: Vec<String> = values.iter().map(format_value).collect();
    format!("[{}]", parts.join(", "))
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Int(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Array(values) => format_values(values),
    }
}
