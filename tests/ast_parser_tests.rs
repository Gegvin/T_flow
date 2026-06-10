use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();

    path.pop();

    if path.ends_with("deps") {
        path.pop();
    }

    path.push(if cfg!(windows) { "tflow.exe" } else { "tflow" });

    path
}

fn write_temp_program(name: &str, source: &str) -> PathBuf {
    let mut path = std::env::temp_dir();

    path.push(format!("tflow_ast_{}_{}.tflow", name, std::process::id()));

    fs::write(&path, source).expect("failed to write temp tflow program");

    path
}

fn run_ast(name: &str, source: &str) -> std::process::Output {
    let input_path = write_temp_program(name, source);

    let output = Command::new(get_binary_path())
        .arg("--ast")
        .arg(&input_path)
        .output()
        .unwrap_or_else(|_| panic!("failed to run AST parser on {:?}", input_path));

    fs::remove_file(&input_path).ok();

    output
}

#[test]
fn test_ast_parser_builds_state_node_grid_step() {
    let source = r#"
state x: Int keep(1) = 0 @ [1, 2, 3] #wrap;

node update() {
    let y = x#[0] + 1;
    next x = y;
}

grid main = update[3]();

step {
    run main;
}
"#;

    let output = run_ast("valid_program", source);

    assert!(
        output.status.success(),
        "AST parser failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Program"));
    assert!(stdout.contains("StateDecl"));
    assert!(stdout.contains("NodeDecl"));
    assert!(stdout.contains("GridDecl"));
    assert!(stdout.contains("StepDecl"));
    assert!(stdout.contains("name: \"x\""));
    assert!(stdout.contains("name: \"update\""));
}

#[test]
fn test_ast_parser_builds_binary_expression() {
    let source = r#"
state x: Int keep(1) = 0 @ [1, 2, 3] #wrap;

node update() {
    next x = x#[0] + 10 * 2;
}

grid main = update[3]();

step {
    run main;
}
"#;

    let output = run_ast("binary_expression", source);

    assert!(
        output.status.success(),
        "AST parser failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Binary"));
    assert!(stdout.contains("op: Add"));
    assert!(stdout.contains("op: Mul"));
    assert!(stdout.contains("Spatial"));
}

#[test]
fn test_ast_parser_builds_ternary_expression() {
    let source = r#"
state x: Int keep(1) = 0 @ [1, 2, 3] #wrap;

node update() {
    next x = x#[0] > 1 ? 100 : 0;
}

grid main = update[3]();

step {
    run main;
}
"#;

    let output = run_ast("ternary_expression", source);

    assert!(
        output.status.success(),
        "AST parser failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Ternary"));
    assert!(stdout.contains("cond"));
    assert!(stdout.contains("then_expr"));
    assert!(stdout.contains("else_expr"));
    assert!(stdout.contains("op: Gt"));
}

#[test]
fn test_ast_parser_reports_missing_expression() {
    let source = r#"
state x: Int keep(1) = 0 @ [1, 2, 3] #wrap;

node update() {
    next x = ;
}

grid main = update[3]();

step {
    run main;
}
"#;

    let output = run_ast("missing_expression", source);

    assert!(
        !output.status.success(),
        "AST parser should fail on invalid program"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("expected expression"));
}

#[test]
fn test_ast_parser_reports_missing_semicolon() {
    let source = r#"
state x: Int keep(1) = 0 @ [1, 2, 3] #wrap;

node update() {
    next x = x#[0] + 1
}

grid main = update[3]();

step {
    run main;
}
"#;

    let output = run_ast("missing_semicolon", source);

    assert!(
        !output.status.success(),
        "AST parser should fail on missing semicolon"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("expected ';' after next statement"));
}
