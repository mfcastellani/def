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
| `.body_from(path)`             | Load body from a `.jdef` or `.tdef` file                               |
| `.type(JSON\|TEXT)`            | Set `Content-Type` header (`application/json` or `text/plain`)         |
| `.with_var(variable)`          | Register a variable for template substitution                          |
| `.retries(n)`                  | Retry the request up to `n` times on network failure                   |
| `.fixed_backoff(ms)`           | Wait `ms` milliseconds between retries (constant)                      |
| `.linear_backoff(ms)`          | Wait `ms`, `2×ms`, `3×ms`, … between retries                          |
| `.exponential_backoff(ms)`     | Wait `ms`, `2×ms`, `4×ms`, … between retries                          |
| `.timeout(ms)`                 | Maximum time per attempt in milliseconds                               |
| `.timeout(ms, "message")`      | Maximum time per attempt; show `"message"` on failure                  |
| `.with_mocks(mock_or_array)`   | Intercept matching requests with pre-configured responses              |
| `.inspect()`                   | Print request details to stdout for debugging; returns self            |
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
| `content_type()`        | `string`   | value of the `Content-Type` response header       |
| `header(name)`          | `string`   | value of a specific response header (case-insensitive) |
| `headers()`             | `array`    | all headers as `tuple(name, value)` elements      |
| `json(path)`            | value      | extract a value from a JSON body by path          |
| `json_exists(path)`     | `boolean`  | true when the JSON path exists in the body        |
| `expect(predicate)`     | `response` | assert a condition; returns self for chaining     |
| `inspect()`             | `response` | print response details to stdout; returns self    |

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
