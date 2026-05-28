# Language Reference

## Basic Syntax

The `def` keyword declares variables and functions. Functions do not use `return` — the last evaluated expression in the body is the return value. Comments use `//` and run to the end of the line.

```def
def i     as integer(10)
def price as float(10.5)
def name  as string("Def")
def ok    as boolean(true)
def items as array
def now   as datetime

def sum as function(a as integer, b as integer) (
  a + b
)

def n as integer(sum(10, 12))

assert(n == 22)
```

## Variables and Assignment

Variables are declared with or without an initializer:

```def
def a as integer
a = 10
a += 5
a -= 3

def price as float(10.5)
```

Default values when declared without an initializer:

| Type       | Default                              |
|------------|--------------------------------------|
| `integer`  | `0`                                  |
| `float`    | `0.0`                                |
| `string`   | `""`                                 |
| `boolean`  | `false`                              |
| `array`    | `[]`                                 |
| `tuple`    | `tuple("", nil)`                     |
| `datetime` | current system date and time         |
| `response` | status `0`, empty body, no headers   |

Arithmetic operators (`+`, `-`, `*`, `/`, `%`) and compound assignments (`+=`, `-=`) work on `integer` and `float` only. String concatenation uses `concat(...)`; array mutation uses `push(...)`.

## Types

### integer and float

```def
def roll as integer().random(1, 6)     // random integer 1..6
def prob as float().random(0.0, 1.0)   // random float in [0.0, 1.0]

def n as integer(42)
def s as string(n.to_string())         // s == "42"

def f as float(9.5)
def t as string(f.to_string())         // t == "9.5"
```

| Method             | Returns   | Description                                           |
|--------------------|-----------|-------------------------------------------------------|
| `random(min, max)` | same type | Random value in `[min, max]` (inclusive on both ends) |
| `to_string()`      | `string`  | String representation (`42` → `"42"`)                 |

`float.random` also accepts integer arguments.

### string

String values support template interpolation in `print`. Any valid Def expression works inside `{{...}}`:

```def
def base as string("https://api.example.com")
def url  as string(concat(base, "/posts/", id))

print("status: {{res.describe_status()}}")
print("ok: {{res.ok()}}")
```

String literals cannot be nested inside `{{...}}`; use a variable instead.

### boolean

`true` or `false`. Boolean operators: `and`, `or`, `not`.

### array

```def
def names as array("Marcelo", "Ana", "Nicolas")

names.push("Def")

print(names.len())
print(names.get(0))
print(names[1])
```

| Method            | Returns   | Description                         |
|-------------------|-----------|-------------------------------------|
| `len()`           | `integer` | Number of elements                  |
| `is_empty()`      | `boolean` | True when the array has no elements |
| `get(index)`      | element   | Element at zero-based index         |
| `push(value)`     | —         | Append a value                      |

Index access via `array[n]` is also supported.

### tuple

Key/value pairs. The key must be a `string`; the value may be `string`, `integer`, `float`, or `boolean`. Primary use: passing headers and query parameters to requests.

```def
def age as tuple("Age", 48)

assert(age.key()   == "Age")
assert(age.value() == 48)
```

### datetime

Captures the current system date and time at declaration:

```def
def now as datetime

print(now.format("hh:mm:ss dd/mm/yyyy"))

now.year(2026)
now.month(1)
now.day(1)

print("{{now.day()}}/{{now.month()}}/{{now.year()}}")
```

Format mask tokens:

| Token  | Meaning                                   |
|--------|-------------------------------------------|
| `hh`   | hour (24h)                                |
| `mm`   | minutes (after `hh`), month (after `dd`)  |
| `ss`   | seconds                                   |
| `dd`   | day                                       |
| `yy`   | two-digit year                            |
| `yyyy` | four-digit year                           |

Setter methods (`hour()`, `minute()`, `second()`, `day()`, `month()`, `year()`) accept an integer argument to update the component and return the updated `datetime`.

## Operators

Comparison operators:
- `==` and `!=` — any type
- `>`, `<`, `>=`, `<=` — `integer` and `float` only

Boolean operators:
- `and` — true when both operands are true
- `or` — true when at least one operand is true
- `not` — negates a boolean

## Builtins

`assert(expr)` aborts execution when the expression is false:

```def
assert(1 + 2 == 3)
assert(res.ok())
```

`print(value)` writes to stdout. String literals support `{{expression}}` interpolation:

```def
print("status: {{res.describe_status()}}")
print("duration: {{res.duration()}}ms")
```

`delay(ms)` pauses execution for `ms` milliseconds (integer):

```def
delay(500)
```

`concat(a, b, ...)` joins two or more strings:

```def
def url as string(concat(base, "/users/", id))
```

## Conditionals and Scope

```def
def status  as integer(200)
def message as string()

if status == 200 (
  message = "ok"
) else (
  message = "unexpected"
)

assert(message == "ok")
```

The `else` branch is optional. Blocks (`if`, `for`, `while do`, functions) create a local scope: `def` inside a block is local to it; assignments update the nearest enclosing variable.

## Match

`match` compares a value against literal patterns in order and returns the first match. `_` is the catch-all:

```def
def label as string(
  match status (
    200 => "ok",
    201 => "created",
    404 => "not found",
    _   => "unexpected"
  )
)
```

`match` works as an expression, so it can initialize variables, be used in function bodies, or appear in assignments.

## Loops

### for — iterate over a collection

```def
def items as array(10, 20, 30)
def total as integer(0)

for item in items (
  total += item
)

assert(total == 60)
```

### while do — repeat while a condition is true

```def
def i as integer(1)
def sum as integer(0)

while i <= 100 do (
  sum += i
  i += 1
)

assert(sum == 5050)
```

### break() and next()

`break()` exits the loop immediately. `next()` skips the remaining statements in the current iteration and jumps to the next one. Both work inside `for` and `while do`.

```def
def items as array(1, 2, 3, 4, 5)
def evens as array

for item in items (
  if item % 2 != 0 (
    next()
  )
  evens.push(item)
)

assert(evens.len() == 2)
```

- `break()` and `next()` take no arguments.
- Calling them outside a loop is a runtime error.
- They do not propagate across function call boundaries.

## Functions

```def
def sum as function(a as integer, b as integer) (
  a + b
)

def n as integer(sum(10, 12))
assert(n == 22)
```

The last evaluated expression is the return value. Functions have their own scope and do not see the caller's local variables.
