# Спецификация T-Flow (v0.3.0)

T-Flow — это предметно-ориентированный язык (DSL) для описания высокопроизводительных параллельных систем, работающих на сетках (физические симуляции, клеточные автоматы, нейронные поля).

## 1. Система типов и память

### 1.1 Примитивы
*   `Int`: 64-битное целое со знаком.
*   `Float`: 64-битное число с плавающей точкой.
*   `Bool`: Логическое значение (`true`, `false`).

### 1.2 Структуры и SoA (Structure of Arrays)
Язык использует высокоуровневые структуры для удобства, но компилирует их в SoA-массивы.
```rust
struct Cell { identity: Float, energy: Float }
```
Если `Cell` используется в `grid`, компилятор создаст два отдельных буфера в памяти GPU. Доступ к `cell.energy` будет транслирован в чтение из массива энергий, что оптимизирует использование кэша (Coalesced Memory Access).

### 1.3 Ресурсы (Memory Model)
Все ресурсы делятся на статические, разделяемые и контекстные. Для всех ресурсов с пространственным измерением обязателен спецификатор границы `#`.

| Ресурс | Контекст | Синтаксис | Описание |
| :--- | :--- | :--- | :--- |
| `const` | Global | `const Name = Val;` | Значение подставляется при компиляции. |
| `table` | Read-only | `table T # Bound : T[D] = [..];` | Lookup-таблица (текстурный кэш). |
| `param` | Read/Write | `param P # Bound : T[D] = [..];` | Глобальный буфер, обновляется пошагово. |
| `state` | Local | `state S : T keep(N) = V @ [H] # B;` | Состояние узла с историей (Double Buffering). |

**Обработка границ (`#`):**
*   `fixed(val)`: При выходе за границы возвращает `val`.
*   `wrap`: Тороидальная топология (зацикливание координат).
*   `clamp`: Насыщение (использование значения ближайшей граничной ячейки).

---

## 2. Системный контекст (`self`)

Внутри контекста `node` доступны встроенные переменные `self`:
*   `self.x, self.y, self.z`: Целочисленные координаты текущего вычислительного узла.
*   `self.index`: Линейный индекс узла в памяти.
*   `self.size_x, self.size_y, self.size_z`: Размеры сетки (Grid dimensions).

---

## 3. Вычислительные единицы

### 3.1 Функции (`fn`)
Чистые инлайн-функции. 
*   **Ограничение:** Не могут иметь побочных эффектов или обращаться к `state`.
*   **Цель:** Математические абстракции.

### 3.2 Ноды (`node`)
Ядро параллельных вычислений. Выполняются в SIMD-стиле.
*   **Ограничение:** Запрещены ветвления (`if/else`) и циклы. Разрешен только тернарный оператор `cond ? a : b` (предсказуемость потока команд).
*   **Оператор `next`:** Используется для записи результата в теневой буфер. Значения обновляются только после завершения всего такта.

---

## 4. Грамматика (EBNF)

```ebnf
Program       ::= (Declaration | GridDecl | StepDecl)*
Declaration   ::= StructDecl | ConstDecl | TableDecl | ParamDecl | StateDecl | FnDecl | NodeDecl

(* Ресурсы *)
StateDecl     ::= "state" Ident ":" Type "keep" "(" IntLiteral ")" "=" Expression ("@" FlatArray)? "#" Bound ";"
TableDecl     ::= "table" Ident "#" Bound ":" Type DimList "=" ArrayLiteral ";"
ParamDecl     ::= "param" Ident "#" Bound ":" Type DimList "=" ArrayLiteral ";"

(* Логика *)
NodeDecl      ::= "node" Ident "(" Params? ")" "{" NodeStatement* "}"
NodeStatement ::= LetStmt | NextStmt
NextStmt      ::= "next" AccessTarget ("=" | "+=" | "-=" | "*=" | "/=") Expression ";"

(* Выражения *)
Expression    ::= Ternary
Ternary       ::= LogicOr ("?" Expression ":" Expression)?
Primary       ::= Atom (Selector)*
Selector      ::= Field | History | Space | Index | Call | Method | Fold

History       ::= "@" ("now" | "prev" | "0" | "-" IntLiteral)
Space         ::= "#" "[" Expression ("," Expression)* "]"
Fold          ::= ".fold" "(" Expression "," Ident ")"
```

---

## 5. Приоритет операторов

| Priority | Operator | Associativity | Description |
| :--- | :--- | :--- | :--- |
| 1 | `()` `[]` `.` `#` `@` | Left | Группировка, Индексация, Пространство/Время, Fold |
| 2 | `!` `-` (unary) | Right | Унарные операторы |
| 3 | `as` | Left | Приведение типов |
| 4 | `*` `/` | Left | Мультипликативные |
| 5 | `+` `-` | Left | Аддитивные |
| 6 | `<` `>` `<=` `>=` | Left | Сравнение |
| 7 | `==` `!=` | Left | Равенство |
| 8 | `&&` | Left | Логическое И |
| 9 | `||` | Left | Логическое ИЛИ |
| 10 | `? :` | Right | Тернарный оператор |
| 11 | `=` `+=` `-=` `*=` `/=`| Right | Присваивание (только в `next` / `let`) |

---

## 6. Жизненный цикл и Pipeline

### 6.1 Grid (Инстанцирование)
Определяет топологию и выделяет память под ноды.
```rust
grid World = NCA_Node[1024, 1024](0.5);
```

### 6.2 Step (Управление тактом)
Блок `step` — это императивный дирижер симуляции.
1.  **`run GridName;`** — запускает параллельное вычисление всех нод в сетке.
2.  **Double Buffering:** Все `next` записи внутри `node` попадают в "теневой" слой.
3.  **Swap:** В конце блока `step` (или по завершении `run` для `state`) происходит переключение буферов.
4.  **Global Next:** Оператор `next` внутри `step` позволяет обновлять `param` (например, через `.fold()`).

---

## 7. Пример: Neural Cellular Automata (NCA)

```rust
struct Cell {
    identity: Float,
    energy: Float
}

// Таблица весов свертки
table Kernels # fixed(0.0) : Float[3] = [0.1, 0.8, 0.1];

node NCA_Node(rate: Float) {
    // Начальное состояние: t0 = 1.0, история t-1 = 0.0
    state body : Cell keep(1) = {1.0, 1.0} @ [{0.0, 0.0}] # wrap;
    
    // Сбор энергии соседей (пространственный доступ #)
    let n_energy = body#[-1].energy * Kernels[0] + 
                   body.energy      * Kernels[1] + 
                   body#[1].energy  * Kernels[2];
                   
    let is_active = body.identity > 0.1;
    
    // Обновление состояния
    next body.energy = is_active ? n_energy * rate : 0.0;
}

param TotalEnergy : Float[1] = [0.0];

fn sum_energy(acc: Float, c: Cell) -> Float {
    return acc + c.energy;
}

grid MyWorld = NCA_Node[1024](0.98);

step {
    run MyWorld; 
    // Редукция данных из сетки в параметр
    next TotalEnergy[0] = MyWorld.body.fold(0.0, sum_energy);
}
```

### Ключевые изменения версии 0.2.2:
*   **Явные границы:** Символ `#` теперь обязателен для любого обращения к памяти, имеющей "соседей".
*   **Разделение `@`:** Символ используется и для инициализации истории в `state`, и для доступа к ней в выражениях.
*   **Метод `.fold()`:** Введен синтаксис для редукции данных сетки в скалярные значения внутри `step`.
*   **SoA-first:** Архитектура языка гарантирует, что пользователь пишет в терминах структур, а машина выполняет в терминах потоков данных.