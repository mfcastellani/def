# DefLang

DefLang is a scripting language for API testing, HTTP workflows, mocking and snapshot validation.

Unlike other tools like Postman collections, DefLang workflows are text files that can be versioned, reviewed, tested and executed from CI/CD pipelines.

It also provides a typed, readable syntax for building requests, validating responses, and chaining multiple API calls. 


```def
def user_mock as mock(GET, "https://api.example.com/users/1")
  .header("Content-Type", "application/json")
  .body_from("data/user.json")
  .reply(200)

def res as response(
  request(GET)
    .path("https://api.example.com/users/1")
    .with_mocks(user_mock)
    .do()
)

res.expect(ok)
res.expect(status == 200)
res.expect(duration < 5000)

assert(res.json("$.id") == 1)
assert(res.json("$.name") == "Marcelo")
assert(res.json("$.active") == true)

assert(res.json("$.score") == 9.5)
```

Written in Rust. The language pipeline (lexer, parser, AST, interpreter) is complete and all examples run against live APIs.

## Installation

```bash
cargo install deflang
```

This installs the `def` binary:

```bash
def run workflow.def
def run workflow.def --param env=staging
def check workflow.def        # dry-run syntax check
def help request              # built-in reference
```

Or run directly from a clone:

```bash
cargo run -- run examples/types/integer.def
./examples/run_all.sh         # run all examples (requires network)
./examples/run_all.sh --skip-http
```

## What DefLang Does

- **Typed variables** — `integer`, `float`, `string`, `boolean`, `array`, `tuple`, `datetime`; add `const` for immutable bindings that reject reassignment at runtime; multiline strings with `"""..."""` (auto-dedent)
- **Fluent HTTP client** — build requests with `.path()`, `.header()`, `.body_from()`, `.do()`; inspect responses with `.status()`, `.json()`, `.expect()`
- **Control flow** — `if`/`else`, `for`, `while do`, `match`, user-defined functions; `range(1..10)` for integer ranges
- **Retry and backoff** — `retries(n)` with `fixed`, `linear`, or `exponential` backoff and per-attempt `timeout`
- **Mocks** — intercept HTTP calls with pre-configured responses for testing and offline workflows
- **File templates** — `.hdef` (headers), `.qdef` (query strings), `.jdef` (JSON body), `.tdef` (text body) with `{{variable}}` substitution
- **Imports** — split workflows into modules; share helpers across files
- **Environment variables** — load defaults from `.edef` files, override with system env
- **CLI parameters** — pass typed values at runtime with `--param KEY=VALUE`
- **Dry-run mode** — `def check` validates the full script without making any HTTP calls
- **JSON path** — extract and assert on response fields with `res.json("$.user.name")`
- **Snapshot assertions** — save a response baseline with `.snapshot()` and validate structural consistency on every run with `res.assert_snapshot()`
- **HTML support** — query HTML responses with CSS selectors (`res.html()`, `res.html_all()`, `res.html_attr()`), regex matching (`res.body_matches()`), and submit forms with `.form_from()` and `.fdef` files

## Documentation

| Topic | Description |
|---|---|
| [Language](docs/language.md) | Syntax, variables, types, operators, if/else, match, loops, functions |
| [Requests](docs/requests.md) | HTTP client, builder API, response methods, headers, body, retry, expect, inspect |
| [Mocks](docs/mocks.md) | Intercept requests with pre-configured responses |
| [JSON Assertions](docs/json.md) | `json(path)` and `json_exists(path)` on response bodies |
| [Snapshots](docs/requests.md#snapshots) | Save response baselines and validate structural consistency with `assert_snapshot()` |
| [Imports](docs/imports.md) | Multi-file workflows and module organization |
| [Environment Variables](docs/envvars.md) | `.edef` files and `from_env_var` |
| [Parameters](docs/params.md) | `--param` and `from_cmd_param` |
| [CLI Reference](docs/cli.md) | `run`, `check`, `help` subcommands |

The built-in `def help <topic>` command also covers everything above from the terminal.

## Examples

The `examples/` directory is organized by topic:

```
examples/
├── assertions/        # assert(), json() assertions, and assert_snapshot() for structural validation
├── html/              # HTML scraping with CSS selectors, regex matching, and form submission
├── brazilian_docs/    # CPF and CNPJ generators
├── language/          # control flow, loops, imports, functions
├── types/             # one file per type
├── headers/           # .hdef usage
├── query-string/      # .qdef usage
├── body/              # .jdef and .tdef usage
├── env/               # .edef usage
├── debugging/         # inspect() for request and response
├── mocks/             # mock HTTP responses for testing
└── jsonplaceholder/   # end-to-end workflow against a real API
```

The JSONPlaceholder example (`examples/jsonplaceholder/main.def`) shows a full create/read/list/delete workflow using imports, functions, match, assert, and all response methods.

## License

MIT License — see [LICENSE](LICENSE).
