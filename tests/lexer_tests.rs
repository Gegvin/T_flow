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

    path.push("T-flow");
    path
}

fn run_lexer_test(input_rel_path: &str, expected_rel_path: &str) {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input_path = project_root.join(input_rel_path);
    let expected_path = project_root.join(expected_rel_path);

    let output = Command::new(get_binary_path())
        .arg(&input_path)
        .output()
        .unwrap_or_else(|_| panic!("Failed to execute lexer on {:?}", input_path));

    assert!(output.status.success(), "Lexer failed on {:?}", input_path);

    let out_file = input_path.with_extension("tflow.out");
    let actual = fs::read_to_string(&out_file)
        .unwrap_or_else(|_| panic!("Failed to read output file {:?}", out_file));

    let expected = fs::read_to_string(&expected_path)
        .unwrap_or_else(|_| panic!("Failed to read expected file {:?}", expected_path));

    assert_eq!(
        normalize_newlines(&actual),
        normalize_newlines(&expected),
        "Mismatch in test: {}",
        input_rel_path
    );

    fs::remove_file(&out_file).ok();
}

fn run_lexer_error_test(input_rel_path: &str, expected_stderr_rel_path: &str) {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input_path = project_root.join(input_rel_path);
    let expected_path = project_root.join(expected_stderr_rel_path);

    let output = Command::new(get_binary_path())
        .arg(&input_path)
        .output()
        .unwrap_or_else(|_| panic!("Failed to execute lexer on {:?}", input_path));

    assert!(
        !output.status.success(),
        "Lexer should fail on {:?}",
        input_path
    );

    let actual_stderr = String::from_utf8_lossy(&output.stderr);
    let expected_stderr = fs::read_to_string(&expected_path)
        .unwrap_or_else(|_| panic!("Failed to read expected stderr {:?}", expected_path));

    assert_eq!(
        normalize_newlines(&actual_stderr),
        normalize_newlines(&expected_stderr),
        "Error mismatch in test: {}",
        input_rel_path
    );

    let out_file = input_path.with_extension("tflow.out");
    assert!(
        !out_file.exists(),
        "Error test should not create .tflow.out file"
    );
}

#[test]
fn test_integer_literals() {
    run_lexer_test(
        "tests/lexer/basic/integer.tflow",
        "tests/lexer/basic/integer.expected.out",
    );
}

#[test]
fn test_float_literals() {
    run_lexer_test(
        "tests/lexer/basic/float.tflow",
        "tests/lexer/basic/float.expected.out",
    );
}

#[test]
fn test_operators() {
    run_lexer_test(
        "tests/lexer/basic/operators.tflow",
        "tests/lexer/basic/operators.expected.out",
    );
}

#[test]
fn test_keywords() {
    run_lexer_test(
        "tests/lexer/basic/keywords.tflow",
        "tests/lexer/basic/keywords.expected.out",
    );
}

#[test]
fn test_self_context() {
    run_lexer_test(
        "tests/lexer/basic/self_context.tflow",
        "tests/lexer/basic/self_context.expected.out",
    );
}

#[test]
fn test_float_dot_only() {
    run_lexer_test(
        "tests/lexer/basic/float_dot_only.tflow",
        "tests/lexer/basic/float_dot_only.expected.out",
    );
}

#[test]
fn test_unexpected_percent() {
    run_lexer_error_test(
        "tests/lexer/errors/unexpected_percent.tflow",
        "tests/lexer/errors/unexpected_percent.expected.stderr",
    );
}

#[test]
fn test_caret_operator() {
    run_lexer_error_test(
        "tests/lexer/errors/caret_operator.tflow",
        "tests/lexer/errors/caret_operator.expected.stderr",
    );
}

#[test]
fn test_backtick() {
    run_lexer_error_test(
        "tests/lexer/errors/backtick.tflow",
        "tests/lexer/errors/backtick.expected.stderr",
    );
}
