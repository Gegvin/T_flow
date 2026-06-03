use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn get_project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn get_binary_path() -> PathBuf {
    let mut path = get_project_root();
    path.push("target");
    path.push("release");
    if cfg!(windows) {
        path.push("T-flow.exe");
    } else {
        path.push("T-flow");
    }
    path
}

fn binary_exists() -> bool {
    get_binary_path().exists()
}

fn file_has_errors(file_path: &str) -> bool {
    let full_path = get_project_root().join(file_path);
    if !full_path.exists() {
        return false;
    }

    let source = fs::read_to_string(&full_path).expect("Failed to read file");

    let mut child = match Command::new(get_binary_path())
        .arg("--check")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        let _ = stdin.write_all(source.as_bytes());
    }

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(_) => return false,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.contains("\"message\"")
}

fn require_binary() {
    if !binary_exists() {
        panic!("Binary not found. Run 'cargo build --release' first.");
    }
}

// ========== TESTS ==========

#[test]
fn test_valid_file_parses_correctly() {
    require_binary();
    assert!(
        !file_has_errors("examples/valid.tflow"),
        "Valid file should have no errors"
    );
}

#[test]
fn test_error_missing_comma() {
    require_binary();
    let full_path = get_project_root().join("examples/error_missing_comma.tflow");
    assert!(full_path.exists());
}

#[test]
fn test_error_missing_semicolon() {
    require_binary();
    assert!(
        file_has_errors("examples/error_missing_semicolon.tflow"),
        "Missing semicolon should be detected"
    );
}

#[test]
fn test_error_if_in_node() {
    require_binary();
    assert!(
        file_has_errors("examples/error_if_in_node.tflow"),
        "If statement in node should be detected"
    );
}

#[test]
fn test_error_let_in_step() {
    require_binary();
    assert!(
        file_has_errors("examples/error_let_in_stap.tflow"),
        "Let statement in step should be detected"
    );
}

#[test]
fn test_error_missing_closing_brace_for_node() {
    require_binary();
    assert!(
        file_has_errors("examples/error_missing_closing_brace_for_node.tflow"),
        "Missing closing brace should be detected"
    );
}

#[test]
fn test_error_missing_expression_after_operator() {
    require_binary();
    assert!(
        file_has_errors("examples/error_missing_expression_after_operator.tflow"),
        "Missing expression should be detected"
    );
}

#[test]
fn test_error_missing_parentheses_after_function_name() {
    require_binary();
    assert!(
        file_has_errors("examples/error_missing_parentheses_after_function_name.tflow"),
        "Missing parentheses should be detected"
    );
}

#[test]
fn test_error_missing_return_type_in_function() {
    require_binary();
    assert!(
        file_has_errors("examples/error_missing_return_type_in_function.tflow"),
        "Missing return type should be detected"
    );
}

#[test]
fn test_error_unexpected_symbol() {
    require_binary();
    assert!(
        file_has_errors("examples/error_unexpected_symbol.tflow"),
        "Unexpected symbol should be detected"
    );
}

#[test]
fn test_error_files_dont_crash_parser() {
    require_binary();

    let error_files = [
        "examples/error_if_in_node.tflow",
        "examples/error_let_in_stap.tflow",
        "examples/error_missing_closing_brace_for_node.tflow",
        "examples/error_missing_comma.tflow",
        "examples/error_missing_expression_after_operator.tflow",
        "examples/error_missing_parentheses_after_function_name.tflow",
        "examples/error_missing_return_type_in_function.tflow",
        "examples/error_missing_semicolon.tflow",
        "examples/error_unexpected_symbol.tflow",
    ];

    for file_path in error_files {
        let full_path = get_project_root().join(file_path);
        if full_path.exists() {
            let result = Command::new(get_binary_path())
                .arg("--check")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            assert!(result.is_ok(), "Parser crashed on {}", file_path);
        }
    }
}
