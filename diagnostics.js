const vscode = require("vscode");
const { spawn } = require("child_process");
const path = require("path");
const os = require("os");
const fs = require("fs");

function getBinaryPath(extensionPath) {
    const config = vscode.workspace.getConfiguration("tflow");
    const configured = config.get("binaryPath");
    if (configured && configured.trim() !== "") {
        return configured.trim();
    }

    const bin = os.platform() === "win32" ? "T-flow.exe" : "tflow";

    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (workspaceFolders && workspaceFolders.length > 0) {
        const candidate = path.join(
            workspaceFolders[0].uri.fsPath,
            "target", "release", bin
        );
        if (fs.existsSync(candidate)) {
            return candidate;
        }
    }

    return path.join(extensionPath, "bin", bin);
}

function runChecker(binaryPath, source) {
    return new Promise((resolve) => {
        let stdout = "";
        let stderr = "";
        let launched = true;

        const proc = spawn(binaryPath, ["--check"], {
            stdio: ["pipe", "pipe", "pipe"],
        });

        proc.stdout.on("data", (chunk) => { stdout += chunk.toString(); });
        proc.stderr.on("data", (chunk) => { stderr += chunk.toString(); });

        proc.on("error", (err) => {
            launched = false;
            resolve({ errors: null, launchError: err.message });
        });

        proc.on("close", () => {
            if (!launched) return;
            try {
                const errors = JSON.parse(stdout);
                resolve({ errors, launchError: null });
            } catch {
                console.error("[tflow] unexpected checker output:", stdout, stderr);
                resolve({ errors: [], launchError: null });
            }
        });

        proc.stdin.write(source, "utf8");
        proc.stdin.end();
    });
}

let binaryWarningShown = false;

async function validateDocument(document, collection, extensionPath) {
    if (document.languageId !== "tflow") return;

    const binaryPath = getBinaryPath(extensionPath);
    const source = document.getText();

    const { errors, launchError } = await runChecker(binaryPath, source);

    if (launchError !== null) {
        if (!binaryWarningShown) {
            binaryWarningShown = true;
            vscode.window.showWarningMessage(
                `T-Flow: не удалось запустить анализатор (${launchError}). ` +
                `Укажите путь к бинарнику в настройке tflow.binaryPath или выполните ` +
                `cargo build --release в корне Rust-проекта.`
            );
        }
        return;
    }

    const diagnostics = errors.map((err) => {
        const line = Math.max(0, (err.line ?? 1) - 1);
        const col  = Math.max(0, (err.column ?? 1) - 1);

        const range = new vscode.Range(line, col, line, col + 1);
        const diag  = new vscode.Diagnostic(
            range,
            err.message,
            vscode.DiagnosticSeverity.Error
        );
        diag.source = "tflow";
        return diag;
    });

    collection.set(document.uri, diagnostics);
}

function createDiagnosticsProvider(collection, extensionPath) {
    const validate = (doc) => validateDocument(doc, collection, extensionPath);

    vscode.workspace.textDocuments.forEach(validate);

    const onOpen   = vscode.workspace.onDidOpenTextDocument(validate);
    const onSave   = vscode.workspace.onDidSaveTextDocument(validate);
    const onChange = vscode.workspace.onDidChangeTextDocument((e) => validate(e.document));
    const onClose  = vscode.workspace.onDidCloseTextDocument((doc) => {
        collection.delete(doc.uri);
    });

    return vscode.Disposable.from(onOpen, onSave, onChange, onClose);
}

module.exports = { createDiagnosticsProvider };