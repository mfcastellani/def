# Text Files

The `file` type provides read, write, and append access to text files on disk.

## Declaration

```def
def log   as file(WRITE).path("output/log.txt")
def src   as file(READ).path("data/input.txt")
def notes as file(APPEND).path("notes.txt")
```

The mode is set once at declaration and cannot be changed:

| Mode     | Behaviour                                              |
|----------|--------------------------------------------------------|
| `READ`   | Open an existing file for reading                      |
| `WRITE`  | Create or truncate the file, then write                |
| `APPEND` | Create if absent, then write at the end               |

`.path(string)` sets the file path. Relative paths are resolved from the directory of the running `.def` file.

## Opening and Closing

A file must be opened before any read or write operation, and closed when done:

```def
def f as file(WRITE).path("hello.txt")
f.open()
f.write("Hello, World!\n")
f.close()
```

| Method    | Description                                                  |
|-----------|--------------------------------------------------------------|
| `open()`  | Open the file; error if already open or path is missing      |
| `close()` | Flush buffered data and close the file                       |
| `flush()` | Write buffered data to disk without closing                  |

## Reading

```def
def f as file(READ).path("data.txt")
f.open()

// Read the whole file at once
def content as string(f.read_all())
print(content)

f.close()
```

```def
def f as file(READ).path("data.txt")
f.open()

// Read line by line
def line as string
while f.eof() == false do (
  line = f.read_line()
  print(line)
)

f.close()
```

Using `not`

```def
def f as file(READ).path("data.txt")
f.open()

// Read line by line
def line as string
while not f.eof() do (
  line = f.read_line()
  print(line)
)

f.close()
```


| Method        | Returns   | Description                                              |
|---------------|-----------|----------------------------------------------------------|
| `eof()`       | `boolean` | `true` when the end of the file has been reached         |
| `read_line()` | `string`  | Read one line; trailing `\n` / `\r\n` is stripped       |
| `read_all()`  | `string`  | Read the entire remaining content as a single string     |

`read_line()` returns an empty string and sets `eof()` to `true` when there is nothing left to read.

## Writing

```def
def f as file(WRITE).path("report.txt")
f.open()

f.write("Name: Marcelo\n")
f.write("Score: 100\n")
f.flush()   // optional mid-session flush

f.close()
```

```def
def f as file(APPEND).path("log.txt")
f.open()
f.write("New entry appended.\n")
f.close()
```

| Method           | Description                                             |
|------------------|---------------------------------------------------------|
| `write(string)`  | Write a string to the file (buffered)                   |
| `flush()`        | Flush the write buffer to disk (file stays open)        |

`write` and `flush` are available in `WRITE` and `APPEND` modes only. `close()` always flushes before closing.

## String Escape Sequences

String literals support the following escape sequences, which is particularly useful when writing file content:

| Sequence | Character        |
|----------|------------------|
| `\n`     | Newline          |
| `\t`     | Tab              |
| `\r`     | Carriage return  |
| `\\`     | Backslash        |
| `\"`     | Double quote     |

Alternatively, use triple-quoted multiline strings for content that spans multiple lines:

```def
f.write("""
Line one
Line two
Line three
""")
```

## Examples

See `examples/text_files/` for runnable scripts:

| File             | Demonstrates                          |
|------------------|---------------------------------------|
| `read_file.def`  | Line-by-line reading with `eof()`     |
| `read_all.def`   | Reading an entire file at once        |
| `write_file.def` | Creating a file and writing content   |
| `append_file.def`| Appending lines to an existing file   |
