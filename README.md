# DefLang

DefLang is a scripting language for HTTP workflows. It provides a typed, readable syntax for building requests, validating responses, and chaining multiple API calls — designed as a programmable alternative to tools like Postman or curl scripts.

```def
def jsonplaceholder as imported("jsonplaceholder")

def res as response(jsonplaceholder.create_post())

assert(res.ok())
print("status:   {{res.describe_status()}}")
print("duration: {{res.duration()}}ms")
print("body:     {{res.body()}}")
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

- **Typed variables** — `integer`, `float`, `string`, `boolean`, `array`, `tuple`, `datetime`
- **Fluent HTTP client** — build requests with `.path()`, `.header()`, `.body_from()`, `.do()`; inspect responses with `.status()`, `.json()`, `.expect()`
- **Control flow** — `if`/`else`, `for`, `while do`, `match`, user-defined functions
- **Retry and backoff** — `retries(n)` with `fixed`, `linear`, or `exponential` backoff and per-attempt `timeout`
- **Mocks** — intercept HTTP calls with pre-configured responses for testing and offline workflows
- **File templates** — `.hdef` (headers), `.qdef` (query strings), `.jdef` (JSON body), `.tdef` (text body) with `{{variable}}` substitution
- **Imports** — split workflows into modules; share helpers across files
- **Environment variables** — load defaults from `.edef` files, override with system env
- **CLI parameters** — pass typed values at runtime with `--param KEY=VALUE`
- **Dry-run mode** — `def check` validates the full script without making any HTTP calls
- **JSON path** — extract and assert on response fields with `res.json("$.user.name")`

## Documentation

| Topic | Description |
|---|---|
| [Language](docs/language.md) | Syntax, variables, types, operators, if/else, match, loops, functions |
| [Requests](docs/requests.md) | HTTP client, builder API, response methods, headers, body, retry, expect, inspect |
| [Mocks](docs/mocks.md) | Intercept requests with pre-configured responses |
| [JSON Assertions](docs/json.md) | `json(path)` and `json_exists(path)` on response bodies |
| [Imports](docs/imports.md) | Multi-file workflows and module organization |
| [Environment Variables](docs/envvars.md) | `.edef` files and `from_env_var` |
| [Parameters](docs/params.md) | `--param` and `from_cmd_param` |
| [CLI Reference](docs/cli.md) | `run`, `check`, `help` subcommands |

The built-in `def help <topic>` command also covers everything above from the terminal.

## Examples

The `examples/` directory is organized by topic:

```
examples/
├── assertions/        # json() and json_exists() assertions on response bodies
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
