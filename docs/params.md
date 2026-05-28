# Command-line Parameters

Scripts can accept runtime parameters so you don't have to edit the file between runs. Pass them with `--param KEY=VALUE` (repeatable):

```bash
def run create_user.def --param cpf=999.000.111-00 --param name="João Silva"
```

## Reading Parameters

Use `from_cmd_param("key", default)` to read a parameter inside a script. The **type of the default value** determines how the CLI string is parsed:

```def
def cpf        as string(from_cmd_param("cpf",    "000.000.000-00"))
def count      as integer(from_cmd_param("count",  0))
def price      as float(from_cmd_param("price",    9.99))
def active     as boolean(from_cmd_param("active", false))
```

For datetime, pass a variable or expression as the default:

```def
def now         as datetime
def report_date as datetime(from_cmd_param("report_date", now))
```

Datetime parameters accept RFC3339 (`2026-01-15T10:30:00+00:00`) or date-only (`2026-01-15`) format.

## Required Parameters

If no default is provided and the parameter is not passed, execution aborts with a clear error:

```def
def cpf as string(from_cmd_param("cpf"))
// runtime error: required param 'cpf' not provided — pass --param cpf=<value>
```

## Type Errors

If the parameter value cannot be parsed into the target type, execution aborts:

```bash
def run script.def --param count=batata
# runtime error: cannot parse param 'count' value "batata" as integer
```

## Scope

Parameters are available to the whole script including all imported modules.

The `--param` flag works with both `run` and `check`:

```bash
def check create_user.def --param cpf=999.000.111-00
```
