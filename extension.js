const vscode = require("vscode");
const { createDiagnosticsProvider } = require("./diagnostics");

const hoverTexts = {
    struct: "Structure with fields.",
    const: "Constant value.",
    table: "Read-only table data.",
    param: "Configurable parameter.",
    state: "State value with history.",
    keep: "Number of saved previous states.",
    fn: "Function declaration.",
    node: "Parallel update block.",
    grid: "Grid declaration.",
    step: "Simulation step block.",
    run: "Runs a node in step.",
    let: "Local variable.",
    next: "Writes value to the next state.",
    return: "Returns value from function.",
    as: "Type cast.",

    Int: "Integer type.",
    Float: "Floating type.",
    Bool: "Boolean type.",

    true: "Boolean true.",
    false: "Boolean false.",
    now: "Current state layer.",
    prev: "Previous state layer.",
    self: "Current node context.",

    wrap: "Wrap boundary mode.",
    clamp: "Clamp boundary mode.",
    fixed: "Fixed boundary value."
};

const symbolHoverTexts = {
    "@": "Access to state history.",
    "#": "Spatial access or boundary mode.",
    "?": "Ternary conditional operator."
};

function activate(context) {
    const hoverProvider = vscode.languages.registerHoverProvider("tflow", {
        provideHover(document, position) {
            const range = document.getWordRangeAtPosition(
                position,
                /[#@?]|\b[A-Za-z_][A-Za-z0-9_]*\b/
            );

            if (!range) {
                return undefined;
            }

            const word = document.getText(range);
            const message = hoverTexts[word] || symbolHoverTexts[word];

            if (!message) {
                return undefined;
            }

            const markdown = new vscode.MarkdownString();
            markdown.appendMarkdown(`**${word}**\n\n${message}`);

            return new vscode.Hover(markdown, range);
        }
    });
    const diagnosticCollection = vscode.languages.createDiagnosticCollection("tflow");
    context.subscriptions.push(diagnosticCollection);
    context.subscriptions.push(
        createDiagnosticsProvider(diagnosticCollection, context.extensionPath)
    );

    const formatterProvider = vscode.languages.registerDocumentFormattingEditProvider("tflow", {
        provideDocumentFormattingEdits(document) {
            const text = document.getText();
            const formatted = formatTFlow(text);

            const fullRange = new vscode.Range(
                document.positionAt(0),
                document.positionAt(text.length)
            );

            return [vscode.TextEdit.replace(fullRange, formatted)];
        }
    });

    const definitionProvider = vscode.languages.registerDefinitionProvider("tflow", {
        provideDefinition(document, position) {
            const wordRange = document.getWordRangeAtPosition(
                position,
                /\b[A-Za-z_][A-Za-z0-9_]*\b/
            );

            if (!wordRange) {
                return undefined;
            }

            const word = document.getText(wordRange);
            const text = document.getText();

            const definitionRegex = new RegExp(
                `\\b(fn|node)\\s+${escapeRegExp(word)}\\s*\\(`,
                "g"
            );

            const match = definitionRegex.exec(text);

            if (!match) {
                return undefined;
            }

            const wordStart = match.index + match[0].indexOf(word);
            const positionStart = document.positionAt(wordStart);

            return new vscode.Location(document.uri, positionStart);
        }
    });

    context.subscriptions.push(
        hoverProvider,
        formatterProvider,
        definitionProvider
    );
}

function formatTFlow(text) {
    const lines = text.split(/\r?\n/);
    const result = [];

    let indentLevel = 0;
    const indent = "    ";

    for (const line of lines) {
        const trimmed = line.trim();

        if (trimmed === "") {
            result.push("");
            continue;
        }

        if (trimmed.startsWith("}")) {
            indentLevel = Math.max(0, indentLevel - 1);
        }

        result.push(indent.repeat(indentLevel) + trimmed);

        if (isBlockStart(trimmed)) {
            indentLevel += 1;
        }
    }

    return result.join("\n");
}

function isBlockStart(line) {
    const codePart = line.replace(/\/\/.*$/, "").trim();

    return codePart.endsWith("{");
}

function escapeRegExp(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function deactivate() {}

module.exports = {
    activate,
    deactivate
};