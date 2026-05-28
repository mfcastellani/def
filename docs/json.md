# JSON Assertions

`response.json(path)` extracts a value from a JSON response body using a JSONPath-like syntax. `response.json_exists(path)` checks whether a path is present, returning `false` (not an error) when it is absent.

```def
assert(res.json("$.id") == 1)
assert(res.json("$.user.name") == "Marcelo")
assert(res.json("$.items[0].active") == true)
assert(res.json_exists("$.token") == false)
assert(not res.json_exists("$.error"))
```

## Path Syntax

| Pattern        | Example              | Description                       |
|----------------|----------------------|-----------------------------------|
| `$`            | `$`                  | Root of the document              |
| `$.field`      | `$.id`               | Field access                      |
| `$.a.b`        | `$.user.name`        | Nested field access               |
| `$.a[n]`       | `$.items[0]`         | Array element by zero-based index |
| `$.a[n].field` | `$.users[1].email`   | Combined array and field access   |

Field names accept letters, digits, `_`, and `-`. Array indices must be non-negative integers.

## Return Types

| JSON type    | Def type                |
|--------------|-------------------------|
| string       | `string`                |
| integer      | `integer`               |
| float        | `float`                 |
| boolean      | `boolean`               |
| null         | `nil`                   |
| object/array | `string` (compact JSON) |

## Errors

| Condition                      | Behaviour                                        |
|--------------------------------|--------------------------------------------------|
| Body is not valid JSON         | `runtime error: response body is not valid JSON` |
| Path does not follow syntax    | `runtime error: invalid json path '...'`         |
| Path not found (`json`)        | `runtime error: json path '...' not found`       |
| Path not found (`json_exists`) | returns `false` — no error                       |

## Limitations (current MVP)

- No filter expressions (`[?(...)]`)
- No wildcards (`*`)
- No array slices (`[0:3]`)
- No recursive descent (`..`)
- Objects and arrays are returned as a compact JSON string
