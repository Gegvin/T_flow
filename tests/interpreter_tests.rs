use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

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

    path.push(format!("tflow_{}_{}.tflow", name, std::process::id()));

    fs::write(&path, source).expect("failed to write temp tflow program");

    path
}

fn run_interpreter(name: &str, source: &str, steps: usize) -> String {
    let input_path = write_temp_program(name, source);

    let output = Command::new(get_binary_path())
        .arg("--run")
        .arg(&input_path)
        .arg("--steps")
        .arg(steps.to_string())
        .output()
        .unwrap_or_else(|_| panic!("failed to run interpreter on {:?}", input_path));

    fs::remove_file(&input_path).ok();

    assert!(
        output.status.success(),
        "interpreter failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_increment_state() {
    let source = r#"
state x: Int keep(1) = 0 @ [1, 2, 3] #wrap;

node update() {
    next x = x#[0] + 1;
}

grid main = update[3]();

step {
    run main;
}
"#;

    let actual = run_interpreter("increment_state", source, 3);

    let expected = r#"step 0
x = [1, 2, 3]
step 1
x = [2, 3, 4]
step 2
x = [3, 4, 5]
step 3
x = [4, 5, 6]
"#;

    assert_eq!(normalize_newlines(&actual), normalize_newlines(expected));
}

#[test]
fn test_wrap_boundary() {
    let source = r#"
state x: Int keep(1) = 0 @ [10, 20, 30] #wrap;

node update() {
    next x = x#[-1];
}

grid main = update[3]();

step {
    run main;
}
"#;

    let actual = run_interpreter("wrap_boundary", source, 1);

    let expected = r#"step 0
x = [10, 20, 30]
step 1
x = [30, 10, 20]
"#;

    assert_eq!(normalize_newlines(&actual), normalize_newlines(expected));
}

#[test]
fn test_clamp_boundary() {
    let source = r#"
state x: Int keep(1) = 0 @ [10, 20, 30] #clamp;

node update() {
    next x = x#[-1];
}

grid main = update[3]();

step {
    run main;
}
"#;

    let actual = run_interpreter("clamp_boundary", source, 1);

    let expected = r#"step 0
x = [10, 20, 30]
step 1
x = [10, 10, 20]
"#;

    assert_eq!(normalize_newlines(&actual), normalize_newlines(expected));
}

#[test]
fn test_fixed_boundary() {
    let source = r#"
state x: Int keep(1) = 0 @ [10, 20, 30] #fixed(0);

node update() {
    next x = x#[-1];
}

grid main = update[3]();

step {
    run main;
}
"#;

    let actual = run_interpreter("fixed_boundary", source, 1);

    let expected = r#"step 0
x = [10, 20, 30]
step 1
x = [0, 10, 20]
"#;

    assert_eq!(normalize_newlines(&actual), normalize_newlines(expected));
}

#[test]
fn test_ternary_expression() {
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

    let actual = run_interpreter("ternary_expression", source, 1);

    let expected = r#"step 0
x = [1, 2, 3]
step 1
x = [0, 100, 100]
"#;

    assert_eq!(normalize_newlines(&actual), normalize_newlines(expected));
}
