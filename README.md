# DefLang

DefLang is a scripting language for HTTP workflows. It provides a typed, readable syntax for building requests, validating responses, and chaining multiple API calls,  designed as a programmable alternative to tools like Postman.

```def
def jsonplaceholder as imported("jsonplaceholder")

def res as response(jsonplaceholder.create_post())

assert(res.ok())
print("status:   {{res.describe_status()}}")
print("duration: {{res.duration()}}ms")
print("body:     {{res.body()}}")
```

Written in Rust. Currently at an early but functional stage: the language pipeline (lexer, parser, AST, interpreter) is complete and all examples run against live APIs.

## Running

Install from [crates.io](https://crates.io/crates/deflang):

```bash
cargo install deflang
```

This installs the `def` binary. Then run any `.def` file:

```bash
def examples/types/integer.def
```

Or run directly from a clone of the repository:

```bash
cargo run -- examples/types/integer.def
```

Run all examples (requires network for the HTTP ones):

```bash
./examples/run_all.sh
# or, to skip HTTP calls:
./examples/run_all.sh --skip-http
```

Run the test suite:

```bash
cargo test
```

## Dry-run / Syntax Check

Add `--check` after the file path to run the script in dry-run mode: the full interpreter executes, but HTTP calls return a stub `200` response instead of hitting the network. `print()` and `delay()` are suppressed. Imports are loaded and validated recursively.

This catches syntax errors, undefined variables, unknown methods, wrong argument counts, and type errors — everything except assertions on HTTP response values.

```bash
def workflow.def --check
# workflow.def: syntax ok

def broken.def --check
# runtime error: unknown request method 'retry' at line 5 in 'broken.def'
```

All errors include the line number and file name:

```
parser error: expected ')' after variable initializer at line 5 in 'workflow.def'
runtime error: undefined identifier 'base_url' at line 12 in 'helpers.def'
```

## Basic Syntax

The `def` keyword declares variables and functions. Functions do not use `return`, the last evaluated expression in the body is the return value.

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

Comments use `//` and run to the end of the line.

## Builtins

`assert(boolean_expression)` aborts execution when the expression is false:

```def
assert(1 + 2 == 3)
```

`print(value)` writes a value to stdout. When given a string literal, `{{expression}}` placeholders are evaluated and interpolated:

```def
def res as response(request(GET).path("https://httpbingo.org/anything").do())

print("status:   {{res.describe_status()}}")
print("ok:       {{res.ok()}}")
print("duration: {{res.duration()}}ms")
print("size:     {{res.size()}} bytes")
```

Any valid Def expression works inside `{{...}}` like method calls, arithmetic, function calls, variables. String literals cannot be nested inside `{{...}}`; use a variable instead:

```def
def search as string("language")
print("body contains '{{search}}': {{res.body_contains(search)}}")
```

`delay(ms)` pauses execution for `ms` milliseconds. The argument must be an `integer`.

`concat(a, b, ...)` joins two or more strings. Useful for building URLs:

```def
def base as string("https://api.example.com")
def url  as string(concat(base, "/posts/", id))
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

Comparison operators:

- `==` and `!=` work on any type.
- `>`, `<`, `>=`, `<=` work on `integer` and `float`.

Boolean operators:

- `and` true when both operands are equal (`true and true`, or `false and false`).
- `or` true when at least one operand is true.
- `not` negates a boolean.

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

Blocks (`if`, `for`, functions) create a local scope. A `def` inside a block is local to it. Assignments update the nearest enclosing variable, local if it exists, global otherwise.

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

`match` can be used as a function body, giving a clean way to map integer codes to strings for example:

```def
def http_label as function(code as integer) (
  match code (
    200 => "200 OK",
    201 => "201 Created",
    404 => "404 Not Found",
    _   => "unknown"
  )
)
```

## Arrays

```def
def names as array("Marcelo", "Ana", "Nicolas")

names.push("Def")

print(names.len())
print(names.get(0))
print(names[1])

for name in names (
  print(name)
)
```

Array methods: `len()`, `is_empty()`, `get(index)`, `push(value)`. Index access via `array[n]` is also supported.

## Tuples

Tuples are key/value pairs. The key must be a `string`; the value may be `string`, `integer`, `float`, or `boolean`.

```def
def age as tuple("Age", 48)

assert(age.key()   == "Age")
assert(age.value() == 48)
```

Tuples are the primary way to pass headers and query parameters to requests:

```def
request(GET).header(tuple("Accept", "application/json"))
```

## Date and Time

`datetime` captures the current system date and time at the moment the variable is declared:

```def
def now as datetime

print(now.format("hh:mm:ss dd/mm/yyyy"))
```

Format mask tokens:

| Token  | Meaning                              |
|--------|--------------------------------------|
| `hh`   | hour                                 |
| `mm`   | minutes (after `hh`), month (after `dd`) |
| `ss`   | seconds                              |
| `dd`   | day                                  |
| `yy`   | two-digit year                       |
| `yyyy` | four-digit year                      |

Parts can be read or updated individually. Setters return the updated `datetime`:

```def
def now as datetime

now.year(2026)
now.month(1)
now.day(1)

print("{{now.day()}}/{{now.month()}}/{{now.year()}}")
```

## Imports

`imported` loads a `.def` file into its own isolated context. The path is resolved relative to the current file and the `.def` extension may be omitted.

```def
def math as imported("imports/math")

assert(math.add(10, 12) == 22)

math.variable = "updated"
assert(math.variable == "updated")
```

Imports are the primary way to organize multi-file workflows: define API wrappers in one file, orchestrate calls in another.

## Environment Variables

`.edef` files store environment variable defaults in `name=value` format. `//` and `#` lines are treated as comments.

```edef
# application settings
API_HOST=https://api.example.com
API_KEY=dev-key-1234
TIMEOUT=5000
```

Load the file with `envvars`:

```def
def env as envvars("edef/settings.edef")
```

Read a variable into a string with `from_env_var`:

```def
def host    as string().from_env_var("API_HOST")
def timeout as string().from_env_var("TIMEOUT")
```

**Resolution order**: if the variable is already set in the system environment it takes priority over the `.edef` value, and a warning is printed to stderr:

```
warning: env var 'API_KEY' defined in 'edef/settings.edef' is already set in the system environment — file value ignored
```

This means you can ship safe defaults in `.edef` and override them at runtime without changing the file:

```bash
API_KEY=prod-secret def workflow.def
```

## HTTP Requests

Requests are built with a fluent API and executed with `.do()`, which returns a `response`:

```def
def res as response(
  request(GET)
    .path("https://httpbingo.org/anything")
    .header(tuple("Accept", "application/json"))
    .query_string(tuple("page", "1"))
    .do()
)

assert(res.ok())
print("{{res.describe_status()}} in {{res.duration()}}ms")
```

### Response methods

| Method                  | Returns   | Description                                       |
|-------------------------|-----------|---------------------------------------------------|
| `status()`              | `integer` | HTTP status code                                  |
| `ok()`                  | `boolean` | true when status is 2xx                           |
| `describe_status()`     | `string`  | human-readable status label (`"201 Created"`, …)  |
| `duration()`            | `integer` | round-trip time in milliseconds                   |
| `size()`                | `integer` | response body size in bytes                       |
| `body()`                | `string`  | response body                                     |
| `body_contains(string)` | `boolean` | true when the body contains the given substring   |
| `content_type()`        | `string`  | value of the `Content-Type` response header       |
| `header(name)`          | `string`  | value of a specific header (case-insensitive)     |
| `headers()`             | `array`   | all headers as `tuple(name, value)` elements      |

### Headers

Inline:

```def
request(GET)
  .path("https://httpbingo.org/headers")
  .header(tuple("Accept", "application/json"))
  .header(tuple("Authorization", "Bearer token"))
  .do()
```

From a `.hdef` file with `headers_from(path)`:

```def
request(GET)
  .path("https://httpbingo.org/headers")
  .headers_from("hdef/headers.hdef")
  .with_var(accept_header)
  .do()
```

`.hdef` format: one `Name: value` per line, `//` and `#` comments supported, `{{variable}}` placeholders substituted via `with_var(...)`:

```hdef
// common request headers
Authorization: Bearer token
Accept: {{accept_header}}
```

If the same header appears more than once the last value wins, so `headers_from(...).header(tuple("Authorization", "override"))` replaces the file value.

### Query strings

Inline:

```def
request(GET)
  .path("https://httpbingo.org/anything")
  .query_string(tuple("search", "def language"))
  .query_string(tuple("page", "1"))
  .do()
```

From a `.qdef` file with `query_string_from(path)`:

```def
request(GET)
  .path("https://httpbingo.org/anything")
  .query_string_from("qdef/params.qdef")
  .with_var(search_term)
  .query_string(tuple("page", "2"))
  .do()
```

`.qdef` format: one `name: value` per line, same template and override semantics as headers:

```qdef
// query parameters
search: {{search_term}}
page: 1
```

### Request body

Load a body file with `body_from(path)` and declare its type with `.type(JSON)` or `.type(TEXT)`. Def automatically sets the appropriate `Content-Type` header:

```def
def language as string("Def")

def res as response(
  request(POST)
    .path("https://httpbingo.org/anything")
    .body_from("jdef/body.jdef")
    .type(JSON)
    .with_var(language)
    .do()
)

print("status: {{res.describe_status()}}")
print("body:   {{res.body()}}")
```

`.jdef` is a raw JSON with `{{variable}}` placeholders:

```jdef
{
  "language": "{{language}}",
  "purpose": "HTTP testing"
}
```

`.tdef` is a plain text with `{{variable}}` placeholders. `//` and `#` lines are treated as comments and stripped before sending:

```tdef
// text body
language: {{language}}
purpose: HTTP testing
```

`with_var(variable_name)` registers a string variable for template substitution. It applies to headers, query strings, and body simultaneously. Call order does not matter, all registered variables are applied on every `with_var` call.

### Template variables

The `{{variable}}` substitution system is shared across `.hdef`, `.qdef`, `.jdef`, and `.tdef` files. Only string variables can be registered with `with_var`. For numeric values, declare the variable as a string:

```def
def user_id as string("1")

request(GET)
  .path(concat(base_url, "/users/", user_id))
  .with_var(user_id)
  .do()
```

### Supported HTTP methods

`GET`, `POST`, `PUT`, `PATCH`, `DELETE`, and any other method string accepted by the server.

## Examples

The `examples/` directory is organized by topic:

```
examples/
├── language/          # control flow, imports, functions
├── types/             # one file per type
├── headers/           # .hdef usage
├── query-string/      # .qdef usage
├── body/              # .jdef and .tdef usage
├── env/               # .edef usage
└── jsonplaceholder/   # end-to-end workflow against a real API
```

The JSONPlaceholder example (`examples/jsonplaceholder/main.def`) shows a full create, read, list and delete workflow using imports, functions, match, assert, and all response methods.

## Status

The language core is complete and stable:

- Lexer, parser, AST, and interpreter
- All primitive types: `integer`, `float`, `string`, `boolean`, `array`, `tuple`, `datetime`
- Functions, imports, block scope, `if`/`else`, `for`, `match`
- Full HTTP client with request building, response inspection, and file-based templates
- String interpolation in `print`
- Environment variable loading from `.edef` files with system env precedence
- Error messages include line number and file name
- Dry-run mode (`--check`) for syntax validation without execution

Known limitations:

- Arithmetic operators only support `integer` and `float`.

## Roadmap

- Improve and add standard helpers for response validation.
- Logical operator improvements.

## License

MIT License — see [LICENSE](LICENSE).
