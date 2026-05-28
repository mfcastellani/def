# Environment Variables

`.edef` files store environment variable defaults in `name=value` format. `//` and `#` lines are treated as comments.

```edef
# application settings
API_HOST=https://api.example.com
API_KEY=dev-key-1234
TIMEOUT=5000
```

## Loading

Load the file with `envvars`:

```def
def env as envvars("edef/settings.edef")
```

Read a variable into a string with `from_env_var`:

```def
def host    as string().from_env_var("API_HOST")
def timeout as string().from_env_var("TIMEOUT")
```

## Resolution Order

If the variable is already set in the system environment it takes priority over the `.edef` value, and a warning is printed to stderr:

```
warning: env var 'API_KEY' defined in 'edef/settings.edef' is already set in the system environment — file value ignored
```

This means you can ship safe defaults in `.edef` and override them at runtime without changing the file:

```bash
API_KEY=prod-secret def run workflow.def
```
