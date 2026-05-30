# T-Flow: Domain-Specific Language for Parallel Evolutionary Systems

**T-Flow** — это язык программирования для описания параллельных эволюционных систем (клеточные автоматы, физика частиц, нейронные клеточные автоматы). Язык разработан с акцентом на эффективную компиляцию в GPU-код и оптимизацию кэша через SoA (Structure of Arrays) трансформацию.

## Сборка, тесты и запуск

### Предварительные требования:
    Rust	1.80+
    Cargo	1.80+

```bash
cargo build --release  # сборка проекта
cargo fmt -- --check   # проверка форматирования
cargo fmt              # автоматическое форматирование
cargo test             # запуск тестов
```

Запуск лексера:

```bash
cargo run -- <путь_к_файлу.tflow>
```

Лексер принимает текстовые файлы с расширением .tflow, содержащие код на языке T-Flow и при успешном выполнении создает файл с тем же именем и расширением .tflow.out.

Примеры содержимого .tflow файла и результата работы лексера:

integer.tflow:
```bash
let x = 42;
```

integer.tflow.out:
```bash
Let     "let"   1:1-1:4
Ident   "x"     1:5-1:6
Assign  "="     1:7-1:8
IntLiteral "42" 1:9-1:11
Semicolon ";"   1:11-1:12
```



# Отчет по архитектуре лексера и грамматики T-Flow (v0.3.0)

## 1. Лексический анализ (Lexer)

Лексер реализован с использованием библиотеки `Logos`.
### 1.1 Сводная таблица токенов (Regex)

| Группа | Токен | Регулярное выражение / Правило | Описание |
| :--- | :--- | :--- | :--- |
| **Литералы** | `IntLiteral` | `0x[0-9a-fA-F]+ \| 0b[01]+ \| [0-9]+` | Поддержка Dec, Hex, Bin |
| | `FloatLiteral` | `([0-9]+\.[0-9]*\|[0-9]*\.[0-9]+)([eE][+-]?[0-9]+)?` | Числа с точкой и экспонентой |
| | `BoolLiteral` | `true \| false` | Логические значения |
| | `Ident` | `[a-zA-Z_][a-zA-Z0-9_]*` | Имена переменных и типов |
| **Спецсимволы** | `At` | `@` | Доступ к временному слою (History) |
| | `Hash` | `#` | Пространственный доступ / Границы |
| | `Остальное` | `_` | стандартные символы операций и ключевые слова |

### 1.2 Обработка сложных токенов
1.  **Числа:** Лексер автоматически конвертирует строковые срезы в `i64` и `f64`. При ошибке парсинга (например, число слишком велико) лексер выбрасывает `TFlowError::InvalidToken`.
2.  **Пропуск (Skip):** Пробельные символы (`\t`, `\n`, `\r`) и комментарии вида `// comment` игнорируются на уровне лексера.
3.  **Составные операторы:** Операторы `+=`, `-=`, `*=`, `/=` выделены в отдельные токены для упрощения работы парсера в блоках `next`.

---

## 2. Грамматика (EBNF)

Грамматика T-Flow описывает структуру программы как набор деклараций ресурсов и логических блоков.

```ebnf
(* Программа и верхний уровень *)
Program       ::= (Declaration | GridDecl | StepDecl)*

Declaration   ::= StructDecl | ConstDecl | TableDecl | ParamDecl | StateDecl | FnDecl | NodeDecl

(* Типы данных *)
Type          ::= "Int" | "Float" | "Bool" | Ident

(* Ресурсы *)
StructDecl    ::= "struct" Ident "{" (StructMember ("," StructMember)* (",")?)? "}"
StructMember  ::= Ident ":" Type

ConstDecl     ::= "const" Ident "=" Literal ";"
TableDecl     ::= "table" Ident "#" Bound ":" Type DimList "=" ArrayLiteral ";"
ParamDecl     ::= "param" Ident "#" Bound ":" Type DimList "=" ArrayLiteral ";"
StateDecl     ::= "state" Ident ":" Type "keep" "(" IntLiteral ")" "=" Literal "@" FlatArray "#" Bound ";"

FlatArray     ::= "[" (Literal ("," Literal)* (",")?)? "]"
DimList       ::= ("[" IntLiteral "]")*
Bound         ::= "fixed" "(" Literal ")" | "wrap" | "clamp"

(* Логика *)
FnDecl        ::= "fn" Ident "(" Params? ")" "->" Type "{" Statement* "}"
NodeDecl      ::= "node" Ident "(" Params? ")" "{" NodeStatement* "}"
GridDecl      ::= "grid" Ident "=" Ident "[" (IntLiteral ("," IntLiteral)*)? "]" "(" Args? ")" ";"
StepDecl      ::= "step" "{" StepBodyStmt* "}"
StepBodyStmt       ::= "run" Ident ";" | NextStmt

Statement     ::= LetStmt | ReturnStmt | (Expression ";")
ReturnStmt    ::= "return" Expression ";"

NodeStatement ::= LetStmt | NextStmt
AssignOp      ::= "=" | "+=" | "-=" | "*=" | "/="
NextStmt      ::= "next" AccessTarget AssignOp Expression ";"
LetStmt       ::= "let" Ident "=" Expression ";"

AccessTarget  ::= Ident (Index | Field)*

(* Выражения с учетом приоритетов (сверху вниз) *)
Expression     ::= Ternary

Ternary        ::= LogicOr ("?" Expression ":" Expression)?
LogicOr        ::= LogicAnd ("||" LogicAnd)*
LogicAnd       ::= Comparison ("&&" Comparison)*
Comparison     ::= Addition (("==" | "!=" | "<" | ">" | "<=" | ">=") Addition)*
Addition       ::= Multiplication (("+" | "-") Multiplication)*
Multiplication ::= CastExpr (("*" | "/") CastExpr)*
CastExpr       ::= Unary ("as" Type)*
Unary          ::= ("!" | "-")? Primary

(* Основной элемент: идентификатор или литерал с цепочкой селекторов *)
Primary        ::= Atom (Selector)*

Atom           ::= Literal 
                 | Ident 
                 | ArrayLiteral 
                 | "(" Expression ")"

(* Доступ к данным и вызовы *)
Selector      ::= Field | History | Space | Index | Call | Method

Field         ::= "." Ident
History       ::= "@" ("0" | "-" IntLiteral | "now" | "prev")
Space         ::= "#" "[" IntLiteral ("," IntLiteral)* "]"
Index         ::= "[" Expression "]"
Call          ::= "(" Args? ")"
Method        ::= "." Ident "(" Args? ")"

(* Литералы *)
ArrayLiteral  ::= "[" ((ArrayLiteral | Literal) ("," (ArrayLiteral | Literal))* (",")?)? "]"
Literal       ::= IntLiteral | FloatLiteral | "true" | "false"

Params        ::= Ident ":" Type ("," Ident ":" Type)*
Args          ::= Expression ("," Expression)*

(* Терминалы *)
Ident         ::= [a-zA-Z_][a-zA-Z0-9_]*
IntLiteral    ::= "0x" [0-9a-fA-F]+ | "0b" [01]+ | [0-9]+
FloatLiteral  ::=  ([0-9]+ "." [0-9]* | [0-9]* "." [0-9]+) ([eE][+-]?[0-9]+)?
```

---

## 3. Таблица приоритетов операторов

| Priority | Operator | Associativity | Description |
| :--- | :--- | :--- | :--- |
| 1 | `()` `[]` `.` `#` `@` | Left | Группировка, Индексация, Пространство/Время |
| 2 | `!` `-` (unary) | Right | Логическое НЕ, Унарный минус |
| 3 | `as` | Left | Кастинг типов |
| 4 | `*` `/` | Left | Умножение, Деление |
| 5 | `+` `-` | Left | Сложение, Вычитание |
| 6 | `<` `>` `<=` `>=` | Left | Сравнение |
| 7 | `==` `!=` | Left | Равенство |
| 8 | `&&` | Left | Логическое И |
| 9 | `\|\|` | Left | Логическое ИЛИ |
| 10 | `? :` | Right | Тернарный оператор |
| 11 | `=` `+=` `-=` `*=` `/=` | Right | Присваивание (только в `next` или `let`) |

---

## 4. Особенности семантического анализа

В дополнение к грамматике, парсер и анализатор должны учитывать:

1.  **SoA (Structure of Arrays) Transformation:**
    При обнаружении доступа `state_name.field`, компилятор должен пометить это как чтение из отдельного массива `state_name_field`. Это критично для производительности GPU.
2.  **Node Restrictions:**
    Внутри блока `node` запрещены любые циклы (`for`, `while`) и условные конструкции `if/else`. Ветвление разрешено только через тернарный оператор `cond ? a : b`. Это гарантирует отсутствие расхождения потоков (divergence) в SIMD-вычислениях.
3.  **Next-semantics:**
    Оператор `next` не меняет значение переменной мгновенно. Все изменения записываются в "теневой" буфер и вступают в силу только после завершения такта (Double Buffering).
4.  **Boundary Enforcement:**
    Символ `#` обязателен при доступе к пространственным ресурсам (`table`, `param`, `state`). 


# Вклад в проект
Индусов Никита: Реализация лексера, настройка CI/CD.
Суслов Павел: Определение грамматики, описание токенов.
Лагойда Андрей: написание тестов, система golden files.