# Mocks

Mocks let you intercept HTTP requests and return pre-configured responses without hitting the network. Useful for testing workflows, simulating error scenarios, slow-server behavior, and endpoints that don't exist yet.

## Defining Mocks

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

**Inline mock** — pass directly without declaring a variable first:

```def
def health as response(
  request(GET)
    .path("https://api.example.com/health")
    .with_mocks(mock(GET, "https://api.example.com/health").reply(200, "ok"))
    .do()
)

assert(health.ok())
```

## Mock Methods

| Method                   | Description                                               |
|--------------------------|-----------------------------------------------------------|
| `.reply(status)`         | Set status code; preserves body set by `body_from`        |
| `.reply(status, body)`   | Set status code and inline body string                    |
| `.fail(status)`          | Same as `.reply(status)` — semantic alias for error cases |
| `.fail(status, body)`    | Same as `.reply(status, body)`                            |
| `.header("Name", "val")` | Add a response header inline                              |
| `.headers_from("file")`  | Load response headers from a `.hdef` file                 |
| `.body_from("file")`     | Load response body from a `.jdef` or `.tdef` file         |
| `.with_var(identifier)`  | Register a template variable for file templates           |
| `.delay(ms)`             | Add delay in milliseconds before responding; chainable    |

## Response Headers

```def
// Inline headers
def header_mock as mock(GET, "https://api.example.com/ping")
  .header("X-Service", "mock")
  .header("Cache-Control", "no-cache")
  .reply(200, "pong")
```

The response's `header()` method works exactly as it does for real responses:

```def
assert(ping_res.header("X-Service") == "mock")
```

## Body from File

Use `.body_from()` to load the response body from a `.jdef` (JSON) or `.tdef` (text) template file, using the same `{{variable}}` substitution as request bodies:

```def
// user.jdef: {"id": 1, "name": "{{username}}", "email": "{{email}}"}
// response_headers.hdef: Content-Type: application/json

def username as string("Marcelo")
def email    as string("marcelo@example.com")

def user_mock as mock(GET, "https://api.example.com/users/1")
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
```

## Matching Rules

- Matched by HTTP method (case-insensitive) + URL (exact string)
- If a mock matches but has no `.reply()` or `.fail()` configured → runtime error
- If no mock matches → the real HTTP request is made
- In dry-run (`check`) mode → mocks are skipped; a stub 200 is returned
