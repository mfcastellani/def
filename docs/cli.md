# CLI Reference

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

## run

Executes a `.def` script. HTTP calls are made against the network. Prints errors with the line number and file name:

```bash
def run workflow.def
def run workflow.def --param env=staging --param timeout=3000
```

## check

Runs the script in dry-run mode: the full interpreter executes, but HTTP calls return a stub `200` response instead of hitting the network. `print()` and `delay()` are suppressed. Imports are loaded and validated recursively.

Catches syntax errors, undefined variables, unknown methods, wrong argument counts, and type errors — everything except assertions on HTTP response values.

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

## help

Shows the built-in language reference. Run without a topic to list all available topics:

```bash
def help
def help request
def help loops
def help mocks
```

Available topics: `array`, `assert`, `body`, `check`, `conditionals`, `datetime`, `delay`, `envvars`, `expect`, `float`, `function`, `headers`, `imported`, `inspect`, `integer`, `json`, `loops`, `match`, `mock`, `params`, `query_string`, `request`, `response`, `retry`, `string`, `tuple`.
