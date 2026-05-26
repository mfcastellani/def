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

This installs the `def` binary. Use the `run` subcommand to execute any `.def` file:

```bash
def run examples/types/integer.def
```

Or run directly from a clone of the repository:

```bash
cargo run -- run examples/types/integer.def
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

### Integer and float methods

Both `integer` and `float` support two methods:

| Method              | Returns   | Description                                               |
|---------------------|-----------|-----------------------------------------------------------|
| `random(min, max)`  | same type | Random value in `[min, max]` (inclusive on both ends)     |
| `to_string()`       | `string`  | String representation of the number (`42` → `"42"`)       |

`random` accepts both integer and float arguments for `float.random`. The method is called on a default-initialized value using the empty initializer syntax:

```def
def roll   as integer().random(1, 6)     // random integer 1..6
def prob   as float().random(0.0, 1.0)   // random float in [0.0, 1.0]

def n as integer(42)
def s as string(n.to_string())           // s == "42"

def f as float(9.5)
def t as string(f.to_string())           // t == "9.5"
```

Comparison operators:

- `==` and `!=` work on any type.
- `>`, `<`, `>=`, `<=` work on `integer` and `float`.

Boolean operators:

- `and` true when both operands are true.
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

### Builder methods

| Method                         | Description                                              |
|--------------------------------|----------------------------------------------------------|
| `.path(url)`                   | Set the request URL                                      |
| `.header(tuple(name, value))`  | Add or replace a request header                          |
| `.headers_from(path)`          | Load headers from a `.hdef` file                         |
| `.query_string(tuple(k, v))`   | Append a query parameter                                 |
| `.query_string_from(path)`     | Load query params from a `.qdef` file                    |
| `.body_from(path)`             | Load body from a `.jdef` or `.tdef` file                 |
| `.type(JSON\|TEXT)`            | Set `Content-Type` header (`application/json` or `text/plain`) |
| `.with_var(variable)`          | Register a string variable for template substitution     |
| `.retries(n)`                  | Retry the request up to `n` times on failure             |
| `.fixed_backoff(ms)`           | Wait `ms` milliseconds between retries (constant)        |
| `.linear_backoff(ms)`          | Wait `ms`, `2×ms`, `3×ms`, … between retries            |
| `.exponential_backoff(ms)`     | Wait `ms`, `2×ms`, `4×ms`, … between retries            |
| `.timeout(ms)`                 | Maximum time per attempt in milliseconds                 |
| `.timeout(ms, "message")`      | Maximum time per attempt; show `"message"` on failure    |
| `.inspect()`                   | Print request details to stdout for debugging; returns self |
| `.do()`                        | Send the request and return a `response`                 |

### Response methods

| Method                  | Returns    | Description                                       |
|-------------------------|------------|---------------------------------------------------|
| `status()`              | `integer`  | HTTP status code                                  |
| `ok()`                  | `boolean`  | true when status is 2xx                           |
| `describe_status()`     | `string`   | human-readable status label (`"201 Created"`, …)  |
| `duration()`            | `integer`  | round-trip time in milliseconds                   |
| `size()`                | `integer`  | response body size in bytes                       |
| `body()`                | `string`   | response body                                     |
| `body_contains(string)` | `boolean`  | true when the body contains the given substring   |
| `content_type()`        | `string`   | value of the `Content-Type` response header       |
| `header(name)`          | `string`   | value of a specific header (case-insensitive)     |
| `headers()`             | `array`    | all headers as `tuple(name, value)` elements      |
| `json(path)`            | `value`    | extract a value from a JSON body by path          |
| `json_exists(path)`     | `boolean`  | true when the JSON path exists in the body        |
| `expect(predicate)`     | `response` | assert a condition; returns self for chaining     |
| `inspect()`             | `response` | print response details to stdout for debugging; returns self |

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

`with_var(variable_name)` registers a variable for template substitution. It applies to headers, query strings, and body simultaneously. Call order does not matter, all registered variables are applied on every `with_var` call.

### Template variables

The `{{variable}}` substitution system is shared across `.hdef`, `.qdef`, `.jdef`, and `.tdef` files. `with_var` accepts `string`, `integer`, `float`, and `boolean` — numeric and boolean values are converted to their string representation automatically:

```def
def user_id as integer(1)
def active  as boolean(true)

request(GET)
  .path(concat(base_url, "/users"))
  .with_var(user_id)
  .with_var(active)
  .do()
```

If a `{{placeholder}}` in a template file has no matching registered variable, `.do()` aborts with a clear error:

```
runtime error: header 'Authorization' contains unresolved template variable '{{token}}' — register it with with_var(token)
```

### Retry and Backoff

`retries(n)` re-sends the request up to `n` additional times when it fails due to a network error (connection refused, DNS failure, timeout). HTTP error responses (4xx/5xx) are returned as valid response values and do not trigger retries — use `res.status()` or `res.ok()` to check the result. The backoff strategy controls how long to wait between attempts:

```def
def res as response(
  request(GET)
    .path("https://api.example.com/data")
    .retries(3)
    .exponential_backoff(100)
    .timeout(2000, "service did not respond within 2 seconds")
    .do()
)

assert(res.ok())
```

Backoff strategies (the argument is the base delay in milliseconds):

| Method                    | Delay after attempt 1 | Attempt 2 | Attempt 3 |
|---------------------------|-----------------------|-----------|-----------|
| `fixed_backoff(100)`      | 100ms                 | 100ms     | 100ms     |
| `linear_backoff(100)`     | 100ms                 | 200ms     | 300ms     |
| `exponential_backoff(100)`| 100ms                 | 200ms     | 400ms     |

If more than one backoff strategy is set, only the last one takes effect.

`timeout(ms)` sets the maximum time allowed for each individual attempt. `timeout(ms, "message")` additionally replaces network-level error messages with the given string, useful for user-facing workflows:

```def
request(GET)
  .path("https://api.example.com/health")
  .retries(2)
  .fixed_backoff(500)
  .timeout(1000, "health check timed out")
  .do()
```

### Expect

`expect(predicate)` is a readable alternative to `assert` for response validation. It evaluates the predicate against a set of named response fields and aborts with a descriptive error if it is false. It returns the response, so calls can be chained:

```def
res.expect(ok)
res.expect(status == 200)
res.expect(duration < 5000)

// chainable
res.expect(ok).expect(status == 200).expect(duration < 5000)
```

Fields available inside the predicate:

| Field          | Type      | Description                                   |
|----------------|-----------|-----------------------------------------------|
| `status`       | `integer` | HTTP status code                              |
| `ok`           | `boolean` | true when status is 2xx                       |
| `duration`     | `integer` | round-trip time in milliseconds               |
| `size`         | `integer` | response body size in bytes                   |
| `body`         | `string`  | response body                                 |
| `content_type` | `string`  | value of the `Content-Type` response header   |

When a predicate fails, the error includes the predicate text and current response values:

```
runtime error: expect(status == 201) failed: status=200, ok=true, duration=142ms
```

### Debugging — inspect

`inspect()` prints the full request or response to stdout without interrupting the chain. It is useful when a request behaves unexpectedly and you need to see exactly what was sent and received.

**Request** — call `inspect()` before `.do()` to print method, URL, headers, query parameters, body, template variables, retry configuration, and timeout:

```def
def res as response(
  request(POST)
    .path("https://api.example.com/posts")
    .header(tuple("Authorization", "Bearer token"))
    .body_from("post.jdef")
    .with_var(title)
    .inspect()   // prints everything above, then continues
    .do()
)
```

Output:
```
[inspect] POST https://api.example.com/posts
  headers:
    Authorization: Bearer token
  body:
    {"title": "DefLang post"}
  vars:
    title: DefLang post
```

**Response** — call `inspect()` on a response value to print status, duration, headers, and body:

```def
res.inspect()
```

Output:
```
[inspect] 201 (ok, 142ms)
  headers:
    content-type: application/json
  body:
    {"id": 101, "title": "DefLang post"}
```

Both methods return `self`, so `res.inspect()` can be used as a standalone statement or chained into further method calls.

### Supported HTTP methods

`GET`, `POST`, `PUT`, `PATCH`, `DELETE`, and any other method string accepted by the server.

## CLI

```
Usage: def <command> [file] [--param KEY=VALUE]...

Commands:
  run   <file>   Execute a .def script
  check <file>   Validate without executing HTTP calls (dry-run)
  fmt   <file>   Format a .def script (not yet implemented)
  help  [topic]  Show language help topics

Options:
  --param KEY=VALUE   Pass a named parameter to the script (repeatable)
  --version           Print the version number
  --help              Show this help message
```

## HTTP Error Responses

HTTP responses with 4xx or 5xx status codes are returned as normal `response` values — they do not stop the script. The script can inspect `res.status()` and branch with `if/else`:

```def
def res as response(
  request(POST)
    .path("https://api.example.com/cpf")
    .do()
)

if res.ok() {
  print("CPF registered successfully")
} else {
  def status as integer(res.status())
  print("Request failed with status: {{status}}")
  print("Body: {{res.body()}}")
}
```

Only **network failures** — connection refused, DNS errors, timeouts — cause a request error that stops the script. Use `retries(n)` to retry on those.

| Scenario                    | Result                                      |
|-----------------------------|---------------------------------------------|
| 2xx response                | `Ok` — response value returned              |
| 4xx / 5xx response          | `Ok` — response value returned; check `res.status()` |
| Network error / timeout     | Script stops with a request error           |

## Mocks

Mocks let you intercept HTTP requests and return pre-configured responses without hitting the network. They are useful for testing workflows, error scenarios, slow-server simulation, and endpoints that don't exist yet.

Define a mock with `mock(METHOD, URL)` and configure its response:

```def
// Inline body
def users_mock as mock(GET, "https://api.example.com/users").reply(200, "name: Marcelo")

// Error scenario (semantic alias for .reply())
def error_mock as mock(POST, "https://api.example.com/users").fail(409, "error: conflict")

// Simulated slow response
def slow_mock as mock(GET, "https://slowserver.example.com/data").delay(100).reply(200, "ok")
```

Pass mocks to a request using `.with_mocks()` — accepts a single mock or an array:

```def
def mocks as array(users_mock, error_mock, slow_mock)

def res as response(
  request(GET)
    .path("https://api.example.com/users")
    .with_mocks(mocks)
    .do()
)

assert(res.ok())
assert(res.status() == 200)
assert(res.body_contains("Marcelo"))
```

**Inline mock** — pass a single mock directly without declaring it first:

```def
def health as response(
  request(GET)
    .path("https://api.example.com/health")
    .with_mocks(mock(GET, "https://api.example.com/health").reply(200, "ok"))
    .do()
)

assert(health.ok())
```

### Response headers

Use `.header()` to add individual headers inline, or `.headers_from()` to load them from a `.hdef` file — the same format used for request headers:

```def
// Inline headers
def header_mock as mock(GET, "https://api.example.com/ping")
  .header("X-Service", "mock")
  .header("Cache-Control", "no-cache")
  .reply(200, "pong")

// From a .hdef file (Content-Type: application/json\nX-Request-Id: {{request_id}})
def request_id as string("abc-123")

def json_mock as mock(GET, "https://api.example.com/users/1")
  .with_var(request_id)
  .headers_from("response_headers.hdef")
  .reply(200, "{\"id\": 1}")
```

The response's `header()` method then works exactly as it does for real responses:

```def
assert(ping_res.header("X-Service") == "mock")
assert(json_res.header("Content-Type") == "application/json")
```

### Body from file

Use `.body_from()` to load the response body from a `.jdef` (JSON) or `.tdef` (text) template file, using the same `{{variable}}` substitution as request bodies. Register variables with `.with_var()` before calling `headers_from` or `body_from`, then set the status code with `.reply()`:

```def
// user.jdef: {"id": 1, "name": "{{username}}", "email": "{{email}}"}
// response_headers.hdef: Content-Type: application/json\nX-Request-Id: {{request_id}}

def request_id as string("abc-123")
def username   as string("Marcelo")
def email      as string("marcelo@example.com")

def user_mock as mock(GET, "https://api.example.com/users/1")
  .with_var(request_id)
  .with_var(username)
  .with_var(email)
  .headers_from("response_headers.hdef")
  .body_from("user.jdef")
  .reply(200)

def res as response(
  request(GET)
    .path("https://api.example.com/users/1")
    .with_mocks(user_mock)
    .do()
)

assert(res.ok())
assert(res.body_contains("Marcelo"))
assert(res.header("Content-Type") == "application/json")
assert(res.header("X-Request-Id") == "abc-123")
```

**Mock matching rules:**

- Matched by HTTP method (case-insensitive) + URL (exact string)
- If a mock matches but has no `.reply()` or `.fail()` configured → runtime error
- If no mock matches → the real HTTP request is made
- In dry-run (`check`) mode → mocks are skipped; a stub 200 is returned as usual

**Mock methods:**

| Method | Description |
|---|---|
| `.reply(status)` | Set status code; preserves body set by `body_from` |
| `.reply(status, body)` | Set status code and inline body string |
| `.fail(status)` | Same as `.reply(status)` — semantic alias for error cases |
| `.fail(status, body)` | Same as `.reply(status, body)` |
| `.header("Name", "value")` | Add a response header inline |
| `.headers_from("file.hdef")` | Load response headers from a `.hdef` file |
| `.body_from("file.jdef")` | Load response body from a `.jdef` or `.tdef` file |
| `.with_var(identifier)` | Register a template variable for file templates |
| `.delay(ms)` | Add delay in milliseconds before responding; chainable |

## JSON Assertions

`response.json(path)` extracts a value from a JSON response body using a simple JSONPath-like syntax. `response.json_exists(path)` checks whether a path is present, returning `false` (not an error) when it is absent.

```def
def user_mock as mock(GET, "https://api.example.com/users/1")
  .header("Content-Type", "application/json")
  .body_from("user.json")
  .reply(200)

def res as response(
  request(GET)
    .path("https://api.example.com/users/1")
    .with_mocks(user_mock)
    .do()
)

assert(res.json("$.id") == 1)
assert(res.json("$.user.name") == "Marcelo")
assert(res.json("$.items[0].active") == true)
assert(res.json_exists("$.token") == false)
assert(not res.json_exists("$.error"))
```

### Path syntax

| Pattern        | Example              | Description                       |
|----------------|----------------------|-----------------------------------|
| `$`            | `$`                  | Root of the document              |
| `$.field`      | `$.id`               | Field access                      |
| `$.a.b`        | `$.user.name`        | Nested field access               |
| `$.a[n]`       | `$.items[0]`         | Array element by zero-based index |
| `$.a[n].field` | `$.users[1].email`   | Combined array and field access   |

Field names accept letters, digits, `_`, and `-`. Array indices must be non-negative integers.

### Return types

| JSON type    | Def type                         |
|--------------|----------------------------------|
| string       | `string`                         |
| integer      | `integer`                        |
| float        | `float`                          |
| boolean      | `boolean`                        |
| null         | `nil`                            |
| object/array | `string` (compact JSON)          |

### Errors

| Condition                      | Behaviour                                           |
|--------------------------------|-----------------------------------------------------|
| Body is not valid JSON         | `runtime error: response body is not valid JSON`    |
| Path does not follow syntax    | `runtime error: invalid json path '...'`            |
| Path not found (`json`)        | `runtime error: json path '...' not found`          |
| Path not found (`json_exists`) | returns `false` — no error                          |

### MVP limitations

- No filter expressions (`[?(...)]`)
- No wildcards (`*`)
- No array slices (`[0:3]`)
- No recursive descent (`..`)
- Objects and arrays are returned as a compact JSON string

## Dry-run / Syntax Check

Use `def check` to run the script in dry-run mode: the full interpreter executes, but HTTP calls return a stub `200` response instead of hitting the network. `print()` and `delay()` are suppressed. Imports are loaded and validated recursively.

This catches syntax errors, undefined variables, unknown methods, wrong argument counts, and type errors — everything except assertions on HTTP response values.

```bash
def check workflow.def
# workflow.def: syntax ok

def check broken.def
# runtime error: unknown request method 'retry' at line 5 in 'broken.def'
```

All errors include the line number and file name:

```
parser error: expected ')' after variable initializer at line 5 in 'workflow.def'
runtime error: undefined identifier 'base_url' at line 12 in 'helpers.def'
```

## Command-line Parameters

Scripts can accept runtime parameters so you don't have to edit the file between runs. Pass them with `--param KEY=VALUE` (repeatable):

```bash
def run create_user.def --param cpf=999.000.111-00 --param name="João Silva"
```

Inside the script, use `from_cmd_param("key", default)` to read a parameter. The **type of the default value** determines how the CLI string is parsed:

```def
def cpf   as string(from_cmd_param("cpf",  "000.000.000-00"))
def count as integer(from_cmd_param("count", 0))
def price as float(from_cmd_param("price",  9.99))
def active as boolean(from_cmd_param("active", false))
```

For datetime, pass a variable or expression as the default:

```def
def now as datetime
def report_date as datetime(from_cmd_param("report_date", now))
```

If the parameter is not passed and no default is provided, execution aborts with a clear error:

```def
def cpf as string(from_cmd_param("cpf"))
// runtime error: required param 'cpf' not provided — pass --param cpf=<value>
```

If the parameter value cannot be parsed into the target type, execution aborts:

```bash
def run script.def --param count=batata
# runtime error: cannot parse param 'count' value "batata" as integer
```

Datetime parameters accept RFC3339 (`2026-01-15T10:30:00+00:00`) or date-only (`2026-01-15`) format.

Parameters are available to the whole script including imported modules.

The `--param` flag works with both `run` and `check`:

```bash
def check create_user.def --param cpf=999.000.111-00
```

## Examples

The `examples/` directory is organized by topic:

```
examples/
├── assertions/        # json() and json_exists() assertions on response bodies
├── brazilian_docs/    # CPF and CNPJ generators (random + to_string showcase)
├── language/          # control flow, imports, functions
├── types/             # one file per type
├── headers/           # .hdef usage
├── query-string/      # .qdef usage
├── body/              # .jdef and .tdef usage
├── env/               # .edef usage
├── debugging/         # inspect() for request and response
├── mocks/             # mock HTTP responses for testing
└── jsonplaceholder/   # end-to-end workflow against a real API
```

The JSONPlaceholder example (`examples/jsonplaceholder/main.def`) shows a full create, read, list and delete workflow using imports, functions, match, assert, and all response methods.

## Status

The language core is complete and stable:

- Lexer, parser, AST, and interpreter
- All primitive types: `integer`, `float`, `string`, `boolean`, `array`, `tuple`, `datetime`
- Functions, imports, block scope, `if`/`else`, `for`, `match`
- Full HTTP client with request building, response inspection, and file-based templates
- Native retry with `retries(n)`, configurable backoff (`fixed`, `linear`, `exponential`), and per-attempt `timeout`
- String interpolation in `print`
- Environment variable loading from `.edef` files with system env precedence
- Error messages include line number and file name
- Dry-run mode (`--check`) for syntax validation without execution
- `expect(predicate)` for readable, chainable response assertions
- `json(path)` / `json_exists(path)` for JSONPath-like assertions on response bodies
- `random(min, max)` and `to_string()` methods on `integer` and `float`

Known limitations:

- Arithmetic operators only support `integer` and `float`.

## Roadmap

- Logical operator improvements.

## License

MIT License — see [LICENSE](LICENSE).
