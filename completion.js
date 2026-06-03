const vscode = require("vscode");

const keywords = [
    { label: "struct", kind: vscode.CompletionItemKind.Keyword, detail: "Structure declaration", insertText: "struct ${1:Name} {\n    $0\n}" },
    { label: "const", kind: vscode.CompletionItemKind.Keyword, detail: "Constant declaration", insertText: "const ${1:name} = ${0:value};" },
    { label: "table", kind: vscode.CompletionItemKind.Keyword, detail: "Read-only table data", insertText: "table ${1:name} #${2:wrap}: ${3:Type}${4:[${5:size}]} = [${0}];" },
    { label: "param", kind: vscode.CompletionItemKind.Keyword, detail: "Configurable parameter", insertText: "param ${1:name} #${2:wrap}: ${3:Type}${4:[${5:size}]} = [${0}];" },
    { label: "state", kind: vscode.CompletionItemKind.Keyword, detail: "State with history", insertText: "state ${1:name}: ${2:Float} keep(${3:2}) = ${4:0} @ [${5:0}] #${6:wrap};" },
    { label: "fn", kind: vscode.CompletionItemKind.Keyword, detail: "Function declaration", insertText: "fn ${1:name}(${2:arg}: ${3:Float}) -> ${4:Float} {\n    ${0}\n}" },
    { label: "node", kind: vscode.CompletionItemKind.Keyword, detail: "Parallel node declaration", insertText: "node ${1:name}(${2:cell}: ${3:Cell}) {\n    let ${4:value} = ${5:expression};\n    next ${6:target} = ${4:value};\n    ${0}\n}" },
    { label: "grid", kind: vscode.CompletionItemKind.Keyword, detail: "Grid declaration", insertText: "grid ${1:name} = ${2:Node}[${3:10}, ${4:10}](${5});" },
    { label: "step", kind: vscode.CompletionItemKind.Keyword, detail: "Simulation step", insertText: "step {\n    run ${1:update};\n    ${0}\n}" },
    { label: "run", kind: vscode.CompletionItemKind.Keyword, detail: "Run a node in step", insertText: "run ${1:grid_name};" },
    { label: "let", kind: vscode.CompletionItemKind.Keyword, detail: "Local variable declaration", insertText: "let ${1:name} = ${0:value};" },
    { label: "next", kind: vscode.CompletionItemKind.Keyword, detail: "Write to next state", insertText: "next ${1:target} = ${0:value};" },
    { label: "return", kind: vscode.CompletionItemKind.Keyword, detail: "Return from function", insertText: "return ${0:value};" },
    { label: "as", kind: vscode.CompletionItemKind.Keyword, detail: "Type cast", insertText: "as ${0:Type}" },
];

const types = [
    { label: "Int", kind: vscode.CompletionItemKind.TypeParameter, detail: "Integer type", insertText: "Int" },
    { label: "Float", kind: vscode.CompletionItemKind.TypeParameter, detail: "Floating-point type", insertText: "Float" },
    { label: "Bool", kind: vscode.CompletionItemKind.TypeParameter, detail: "Boolean type", insertText: "Bool" },
];

const literals = [
    { label: "true", kind: vscode.CompletionItemKind.Constant, detail: "Boolean true", insertText: "true" },
    { label: "false", kind: vscode.CompletionItemKind.Constant, detail: "Boolean false", insertText: "false" },
    { label: "now", kind: vscode.CompletionItemKind.Constant, detail: "Current state layer", insertText: "now" },
    { label: "prev", kind: vscode.CompletionItemKind.Constant, detail: "Previous state layer", insertText: "prev" },
    { label: "self", kind: vscode.CompletionItemKind.Constant, detail: "Current context", insertText: "self" },
];

const boundaries = [
    { label: "wrap", kind: vscode.CompletionItemKind.EnumMember, detail: "Wrap boundary mode", insertText: "wrap" },
    { label: "clamp", kind: vscode.CompletionItemKind.EnumMember, detail: "Clamp boundary mode", insertText: "clamp" },
    { label: "fixed", kind: vscode.CompletionItemKind.EnumMember, detail: "Fixed boundary mode", insertText: "fixed(${0:value})" },
];

const snippets = [
    { label: "if-ternary", kind: vscode.CompletionItemKind.Snippet, detail: "Ternary conditional", insertText: "${1:condition} ? ${2:true_value} : ${0:false_value}" },
    { label: "for-loop", kind: vscode.CompletionItemKind.Snippet, detail: "For loop", insertText: "let ${1:i} = ${2:0}; while ${1:i} < ${3:n} { ${0} ${1:i} = ${1:i} + 1; }" },
    { label: "array", kind: vscode.CompletionItemKind.Snippet, detail: "Array literal", insertText: "[${0}]" },
    { label: "index-access", kind: vscode.CompletionItemKind.Snippet, detail: "Array index access", insertText: "${1:array}[${0:index}]" },
    { label: "field-access", kind: vscode.CompletionItemKind.Snippet, detail: "Field access", insertText: "${1:object}.${0:field}" },
];

class TFlowCompletionProvider {
    provideCompletionItems(document, position) {
        const linePrefix = document.lineAt(position).text.substr(0, position.character);
        
        const wordRange = document.getWordRangeAtPosition(position, /[a-zA-Z_][a-zA-Z0-9_]*/);
        let currentWord = "";
        if (wordRange && wordRange.start.character < position.character) {
            currentWord = document.getText(new vscode.Range(wordRange.start, position));
        }

        const allCompletions = [
            ...keywords,
            ...types,
            ...literals,
            ...boundaries,
            ...snippets,
        ];

        const filtered = allCompletions.filter(item => 
            currentWord === "" || item.label.toLowerCase().startsWith(currentWord.toLowerCase())
        );

        return filtered.map(item => {
            const completion = new vscode.CompletionItem(item.label, item.kind);
            completion.detail = item.detail;
            completion.insertText = new vscode.SnippetString(item.insertText);
            
            if (item.label === "state" || item.label === "struct" || item.label === "fn" || item.label === "node") {
                completion.kind = vscode.CompletionItemKind.Snippet;
            }
            
            completion.documentation = new vscode.MarkdownString(`**${item.label}**\n\n${item.detail}`);
            
            return completion;
        });
    }
}

function registerCompletionProvider(context) {
    const provider = vscode.languages.registerCompletionItemProvider(
        "tflow",
        new TFlowCompletionProvider(),
        ".", " ", "(", "[", "{"
    );
    context.subscriptions.push(provider);
}

module.exports = { registerCompletionProvider };