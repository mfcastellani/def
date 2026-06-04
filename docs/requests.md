# HTTP Requests

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

Supported HTTP methods: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, and any method string accepted by the server.

## Builder Methods

| Method                         | Description                                                            |
|--------------------------------|------------------------------------------------------------------------|
| `.path(url)`                   | Set the request URL                                                    |
| `.header(tuple(name, value))`  | Add or replace a request header                                        |
| `.headers_from(path)`          | Load headers from a `.hdef` file                                       |
| `.query_string(tuple(k, v))`   | Append a query parameter                                               |
| `.query_string_from(path)`     | Load query params from a `.qdef` file                                  |
| `.body_from(path)`             | Load body from a `.jdef` or `.tdef` file; infers `Content-Type` automatically |
| `.form_from(path)`             | Load form fields from a `.fdef` file; sets `Content-Type: application/x-www-form-urlencoded` |
| `.type(JSON\|TEXT)`            | Override `Content-Type` header (`application/json` or `text/plain`)    |
| `.with_var(variable)`          | Register a variable for template substitution                          |
| `.retries(n)`                  | Retry the request up to `n` times on network failure                   |
| `.fixed_backoff(ms)`           | Wait `ms` milliseconds between retries (constant)                      |
| `.linear_backoff(ms)`          | Wait `ms`, `2×ms`, `3×ms`, … between retries                          |
| `.exponential_backoff(ms)`     | Wait `ms`, `2×ms`, `4×ms`, … between retries                          |
| `.timeout(ms)`                 | Maximum time per attempt in milliseconds                               |
| `.timeout(ms, "message")`      | Maximum time per attempt; show `"message"` on failure                  |
| `.with_mocks(mock_or_array)`   | Intercept matching requests with pre-configured responses              |
| `.inspect()`                   | Print request details to stdout for debugging; returns self            |
| `.snapshot()`                  | Save a response snapshot to `snapshots/`; skipped if one already exists        |
| `.mock_with_snapshot()`        | Replay response from snapshot if it exists; otherwise execute and save it      |
| `.do()`                        | Send the request and return a `response`                               |

## Response Methods

| Method                  | Returns    | Description                                       |
|-------------------------|------------|---------------------------------------------------|
| `status()`              | `integer`  | HTTP status code                                  |
| `ok()`                  | `boolean`  | true when status is 2xx                           |
| `describe_status()`     | `string`   | human-readable status label (`"201 Created"`, …)  |
| `duration()`            | `integer`  | round-trip time in milliseconds                   |
| `size()`                | `integer`  | response body size in bytes                       |
| `body()`                | `string`   | response body                                     |
| `body_contains(string)` | `boolean`  | true when the body contains the given substring   |
| `body_matches(pattern)` | `boolean`  | true when the body matches the given regex pattern |
| `html(selector)`        | `string`   | text content of the first element matching the CSS selector |
| `html_all(selector)`    | `array`    | text content of all elements matching the CSS selector |
| `html_attr(selector, attr)` | `string` | attribute value of the first element matching the CSS selector |
| `content_type()`        | `string`   | value of the `Content-Type` response header       |
| `header(name)`          | `string`   | value of a specific response header (case-insensitive) |
| `headers()`             | `array`    | all headers as `tuple(name, value)` elements      |
| `json(path)`            | value      | extract a value from a JSON body by path          |
| `json_exists(path)`     | `boolean`  | true when the JSON path exists in the body        |
| `expect(predicate)`     | `response` | assert a condition; returns self for chaining     |
| `inspect()`             | `response` | print response details to stdout; returns self    |
| `assert_snapshot()`     | `response` | assert the response structure matches the saved snapshot; returns self |

## Headers

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

If the same header appears more than once the last value wins, so `.headers_from(...).header(tuple("Authorization", "override"))` replaces the file value.

## Query Strings

Inline:

```def
request(GET)
  .path("https://httpbingo.org/anything")
  .query_string(tuple("search", "def language"))
  .query_string(tuple("page", "1"))
  .do()
```

From a `.qdef` file:

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

## Request Body

Load a body file with `body_from(path)`. The `Content-Type` header is inferred automatically from the file extension: `.jdef` → `application/json`, `.tdef` → `text/plain`. Use `.type(JSON|TEXT)` to override when needed.

```def
def language as string("Def")

def res as response(
  request(POST)
    .path("https://httpbingo.org/anything")
    .body_from("jdef/body.jdef")
    .with_var(language)
    .do()
)
```

`.jdef` — raw JSON with `{{variable}}` placeholders:

```jdef
{
  "language": "{{language}}",
  "purpose": "HTTP testing"
}
```

`.tdef` — plain text with `{{variable}}` placeholders; `//` and `#` lines are stripped before sending:

```tdef
// text body
language: {{language}}
purpose: HTTP testing
```

## Form Body

`.form_from(path)` loads form fields from a `.fdef` file and sends them as `application/x-www-form-urlencoded`. All field values are URL-encoded automatically.

```def
def username as string("alice")

def res as response(
  request(POST)
    .path("https://example.com/login")
    .form_from("login.fdef")
    .with_var(username)
    .do()
)
```

`.fdef` format: one `key: value` per line, `//` and `#` comments supported, `{{variable}}` placeholders substituted via `with_var(...)`:

```fdef
// HTML form fields
username: {{username}}
password: secret123
remember_me: true
```

The resulting request body: `username=alice&password=secret123&remember_me=true`.

## Template Variables

`with_var(variable)` registers a variable for substitution across `.hdef`, `.qdef`, `.jdef`, and `.tdef` files. Accepts `string`, `integer`, `float`, and `boolean` — numeric and boolean values are converted to their string representation automatically. Call order does not matter; all registered variables are applied on every template.

```def
def user_id as integer(1)
def active  as boolean(true)

request(GET)
  .path(concat(base_url, "/users"))
  .with_var(user_id)
  .with_var(active)
  .do()
```

If a `{{placeholder}}` in a template has no matching variable, `.do()` aborts with a clear error:

```
runtime error: header 'Authorization' contains unresolved template variable '{{token}}' — register it with with_var(token)
```

## Retry and Backoff

`retries(n)` re-sends the request up to `n` additional times when it fails due to a network error (connection refused, DNS failure, timeout). HTTP error responses (4xx/5xx) are returned as valid response values and do **not** trigger retries.

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

Backoff strategies (argument = base delay in milliseconds):

| Method                    | Delay after attempt 1 | Attempt 2 | Attempt 3 |
|---------------------------|-----------------------|-----------|-----------|
| `fixed_backoff(100)`      | 100ms                 | 100ms     | 100ms     |
| `linear_backoff(100)`     | 100ms                 | 200ms     | 300ms     |
| `exponential_backoff(100)`| 100ms                 | 200ms     | 400ms     |

If more than one backoff strategy is set, only the last one takes effect.

`timeout(ms, "message")` replaces network-level error messages with the given string, useful for user-facing workflows.

## HTTP Error Responses

| Scenario                    | Result                                                   |
|-----------------------------|----------------------------------------------------------|
| 2xx response                | `Ok` — response value returned                           |
| 4xx / 5xx response          | `Ok` — response value returned; check `res.status()`    |
| Network error / timeout     | Script stops with a request error                        |

4xx/5xx responses do **not** stop the script — inspect `res.status()` and branch with `if/else`:

```def
def res as response(
  request(POST)
    .path("https://api.example.com/users")
    .do()
)

if res.ok() (
  print("created: {{res.describe_status()}}")
) else (
  print("failed: {{res.describe_status()}} — {{res.body()}}")
)
```

## Expect

`expect(predicate)` is a readable alternative to `assert` for response validation. It returns the response for chaining:

```def
res.expect(ok)
res.expect(status == 200)
res.expect(duration < 5000)

// chainable
res.expect(ok).expect(status == 200).expect(duration < 5000)
```

Fields available inside the predicate:

| Field          | Type      | Description                                 |
|----------------|-----------|---------------------------------------------|
| `status`       | `integer` | HTTP status code                            |
| `ok`           | `boolean` | true when status is 2xx                     |
| `duration`     | `integer` | round-trip time in milliseconds             |
| `size`         | `integer` | response body size in bytes                 |
| `body`         | `string`  | response body                               |
| `content_type` | `string`  | value of the `Content-Type` response header |

When a predicate fails, the error includes the predicate text and current response values:

```
runtime error: expect(status == 201) failed: status=200, ok=true, duration=142ms
```

## Debugging — inspect

`inspect()` prints the full request or response to stdout without interrupting the chain.

**Request** — call before `.do()` to print method, URL, headers, query parameters, body, template variables, retry configuration, and timeout:

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

**Response** — call on a response value:

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

Both methods return `self` and can be chained.

## Snapshots

`.snapshot()` records the HTTP **response** (status, headers, and body) as files under a `snapshots/` directory next to the script. The snapshot is only created once — if a snapshot for the same method and URL already exists, the call is silently skipped.

```def
def res as response(
  request(POST)
    .path("https://api.example.com/posts")
    .body_from("post.jdef")
    .snapshot()
    .do()
)
```

Files written (only those that have content):

| File | Contains |
|---|---|
| `snapshots/{name}.sdef` | Response status code |
| `snapshots/{name}.hdef` | Response headers |
| `snapshots/{name}.jdef` | Response body (when `Content-Type: application/json`) |
| `snapshots/{name}.tdef` | Response body (when `Content-Type: text/plain`) |

The name is derived from the HTTP method and URL — for example `POST https://api.example.com/posts` becomes `post-api-example-com-posts-{timestamp}`.

Place `.snapshot()` anywhere in the builder chain before `.do()`. It has no effect in dry-run mode (`def check`).

## Mock with Snapshot

`.mock_with_snapshot()` uses the same snapshot files as `.snapshot()`. On the first run it executes the real request and saves the response; on subsequent runs it loads the saved files and returns the response without making any network call.

```def
def res as response(
  request(POST)
    .path("https://api.example.com/posts")
    .body_from("post.jdef")
    .mock_with_snapshot()
    .do()
)
```

Because `.snapshot()` and `.mock_with_snapshot()` share the same file format and naming, they are fully interchangeable: running `.snapshot()` on an endpoint and then switching to `.mock_with_snapshot()` will replay the existing snapshot immediately.

## Assert Snapshot

`assert_snapshot()` validates that the current response matches the structure of a previously saved snapshot. It is called on a response value after `.do()`.

```def
def res as response(
  request(GET)
    .path("https://api.example.com/users/1")
    .snapshot()   // saves the snapshot on first run
    .do()
)

res.assert_snapshot()   // validates structure on every run
```

**What is validated:**

- **Status code** — must match the snapshot exactly.
- **JSON body structure** — when `Content-Type: application/json`, the field names and types are compared recursively. Values may change freely; types cannot.
- **Text body** — presence or absence of a body must match the snapshot (both empty or both non-empty).

**JSON structural rules:**

| Change | Result |
|--------|--------|
| Field value changes (`"id": 1` → `"id": 2`) | Pass |
| Field type changes (`"id": 1` → `"id": "abc"`) | Fail |
| Field removed from response | Fail |
| Unexpected field added to response | Fail |
| Array becomes empty (or vice versa) | Fail |

When the assertion fails, the error message identifies the exact path and the type mismatch:

```
runtime error: assert_snapshot failed: JSON structure changed
$.user.id: expected number, got string
$.user.email: missing field "email"
```

**First run:** if no snapshot exists for the endpoint, `assert_snapshot()` stops the script with an error. Run `.snapshot()` first to create the baseline.

`assert_snapshot()` returns the response, so it can be chained:

```def
res.assert_snapshot().expect(ok)
```

## HTML Responses

Three response methods support querying HTML bodies using CSS selectors.

### body_matches(pattern)

Tests the body against a regular expression. Works on any body type (HTML, JSON, text).

```def
assert(res.body_matches("<title>.+</title>"))
assert(res.body_matches("\\d{3}-\\d{4}"))   // phone number pattern
assert(not res.body_matches("error"))
```

### html(selector)

Returns the trimmed text content of the **first** element matching the CSS selector. Returns an empty string if no element matches.

```def
def page_title as string(res.html("title"))
def heading   as string(res.html("h1.hero"))
def first_link as string(res.html("nav a"))
```

### html_all(selector)

Returns an array of trimmed text content for **all** elements matching the selector. Declare the variable as `array()` and assign:

```def
def links as array()
links = res.html_all("nav a")

for link in links (
  print("link: {{link}}")
)
```

### html_attr(selector, attribute)

Returns the attribute value of the **first** element matching the selector. Returns an empty string if the element or attribute is not found.

```def
def repo_url  as string(res.html_attr("a.source", "href"))
def logo_src  as string(res.html_attr("img#logo", "src"))
def page_lang as string(res.html_attr("html", "lang"))
```

### Full example

```def
def page_mock as mock(GET, "https://example.com/")
  .header("Content-Type", "text/html; charset=utf-8")
  .body_from("page.html")
  .reply(200)

def res as response(
  request(GET)
    .path("https://example.com/")
    .with_mocks(page_mock)
    .do()
)

assert(res.ok())
assert(res.body_matches("Welcome"))

def title as string(res.html("title"))
assert(title != "")

def items as array()
items = res.html_all("li")

def repo as string(res.html_attr("a.source", "href"))
```
