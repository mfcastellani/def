# Imports

`imported` loads a `.def` file into its own isolated context. The path is resolved relative to the current file; the `.def` extension may be omitted.

```def
def math as imported("imports/math")

assert(math.add(10, 12) == 22)

math.variable = "updated"
assert(math.variable == "updated")
```

Imports are the primary way to organize multi-file workflows: define API wrappers in one file, orchestrate calls in another.

## How It Works

- The imported file runs in a fresh interpreter context with its own variables and functions.
- Accessing `module.name` reads a variable or calls a function defined in the imported file.
- Assigning `module.variable = value` updates a variable in the imported module's context.
- Imports are re-executed on every `imported(...)` call — there is no caching.

## Example Structure

```
workflow.def
jsonplaceholder/
  main.def
  helpers.def
```

`helpers.def`:
```def
def base_url as string("https://jsonplaceholder.typicode.com")

def post_url as function(id as integer) (
  concat(base_url, "/posts/", id.to_string())
)
```

`workflow.def`:
```def
def api as imported("jsonplaceholder/helpers")

def res as response(
  request(GET)
    .path(api.post_url(1))
    .do()
)

assert(res.ok())
```

## Parameters

Parameters passed with `--param` at the command line are available to the whole script including all imported modules.
