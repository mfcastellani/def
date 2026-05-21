use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// Prints `s` as data — braces in `s` are never interpreted as format tokens.
fn text(s: &str) {
    println!("{s}");
}

fn section(title: &str) {
    println!();
    text(title);
    println!();
}

// ── public API ────────────────────────────────────────────────────────────────

pub fn print_version() {
    println!("def {VERSION}");
}

pub fn print_usage() {
    text("Usage: def <file.def> [--check]");
    println!();
    text("Options:");
    text("  --version        Print the version number");
    text("  --help           List all available help topics");
    text("  --help <topic>   Show detailed help for a topic");
    text("  --check          Validate syntax without executing (dry-run)");
    println!();
    text("Run 'def --help' for a full list of topics.");
    text("Run 'def --help about' for information about DefLang.");
}

pub fn print_help() {
    println!("DefLang {VERSION} — a scripting language for HTTP workflows");
    println!();
    text("Usage: def [--version] [--help [topic]] <file.def> [--check]");
    println!();
    text("Topics:");
    text("  array          Ordered collection of values with indexed access and iteration");
    text("  assert         Abort execution when a boolean expression is false");
    text("  body           Request bodies from .jdef (JSON) and .tdef (text) template files");
    text("  check          Validate syntax without executing (dry-run mode)");
    text("  conditionals   if/else control flow with block scope");
    text("  datetime       Current date and time with formatting and individual part access");
    text("  delay          Pause execution for a given number of milliseconds");
    text("  envvars        Load environment variable defaults from .edef files");
    text("  expect         Assert response conditions with readable error messages");
    text("  float          Floating-point numbers and arithmetic operators");
    text("  function       Define and call named user functions");
    text("  headers        Request headers inline or from .hdef template files");
    text("  imported       Load another .def file as a reusable module");
    text("  integer        Integer numbers and arithmetic operators");
    text("  match          Pattern matching against literal values");
    text("  query_string   URL query parameters inline or from .qdef template files");
    text("  request        Build and execute HTTP requests with a fluent API");
    text("  response       Inspect HTTP response status, headers, and body");
    text("  string         Text values and string operations");
    text("  tuple          Key/value pairs used for headers and query parameters");
    println!();
    text("Aliases:");
    text("  hdef                    → headers");
    text("  qdef                    → query_string");
    text("  jdef, tdef, json, text  → body");
    println!();
    text("Run 'def --help <topic>' for details and examples.");
    text("Run 'def --help about' for information about DefLang.");
}

pub fn print_topic(topic: &str) {
    match topic {
        "about" => print_about(),
        "array" => print_array(),
        "assert" => print_assert(),
        "body" | "jdef" | "tdef" | "json" | "text" => print_body(),
        "check" => print_check(),
        "conditionals" => print_conditionals(),
        "datetime" => print_datetime(),
        "delay" => print_delay(),
        "envvars" => print_envvars(),
        "expect" => print_expect(),
        "float" => print_float(),
        "function" => print_function(),
        "headers" | "hdef" => print_headers(),
        "imported" => print_imported(),
        "integer" => print_integer(),
        "match" => print_match(),
        "query_string" | "qdef" => print_query_string(),
        "request" => print_request(),
        "response" => print_response(),
        "string" => print_string(),
        "tuple" => print_tuple(),
        other => {
            eprintln!("unknown help topic '{other}'");
            eprintln!("Run 'def --help' for a list of all topics.");
            process::exit(1);
        }
    }
}

// ── about ─────────────────────────────────────────────────────────────────────

fn print_about() {
    text("DEFLANG");
    section("DESCRIPTION");
    text("  DefLang is a scripting language for HTTP workflows. It provides a typed,");
    text("  readable syntax for building requests, validating responses, and chaining");
    text("  multiple API calls — designed as a programmable alternative to tools like");
    text("  Postman or curl scripts.");
    println!();
    text("  The def keyword declares variables and functions. The language is typed,");
    text("  sequential, and designed to read like a test script. Each .def file is");
    text("  an executable workflow that runs top to bottom.");
    section("LANGUAGE PIPELINE");
    text("  Lexer → Parser → AST → Interpreter");
    text("  Written in Rust.");
    section("TYPES");
    text("  integer    64-bit signed integer");
    text("  float      64-bit floating-point number");
    text("  string     UTF-8 text");
    text("  boolean    true or false");
    text("  array      Ordered collection of values");
    text("  tuple      Key/value pair");
    text("  datetime   Current system date and time");
    text("  request    HTTP request builder");
    text("  response   HTTP response with status, headers, and body");
    section("FEATURES");
    text("  · User-defined functions with typed parameters");
    text("  · Module system via imported()");
    text("  · if/else, for, match control flow with block scope");
    text("  · Full HTTP client: GET, POST, PUT, PATCH, DELETE");
    text("  · Template files for headers (.hdef), query strings (.qdef),");
    text("    JSON bodies (.jdef), and text bodies (.tdef)");
    text("  · Environment variable loading from .edef files");
    text("  · String interpolation in print() with {{expression}} placeholders");
    section("INSTALL");
    text("  cargo install deflang");
    section("SOURCE");
    text("  https://github.com/mfcastellani/def");
    println!();
    text("  by Marcelo Castellani, 2026");
}

// ── topics ────────────────────────────────────────────────────────────────────

fn print_check() {
    text("CHECK  (--check)");
    section("DESCRIPTION");
    text("  Dry-run mode: executes the full script without making real HTTP requests.");
    text("  Catches syntax errors, undefined variables, unknown methods, wrong argument");
    text("  counts, and type errors. HTTP calls return a stub 200 response. print() and");
    text("  delay() are suppressed. Imports are loaded and validated recursively.");
    text("  Exits with code 0 on success, 1 on error.");
    section("SYNTAX");
    text("  def <file.def> --check");
    section("WHAT IS CAUGHT");
    text("  Lexer/parser errors   Syntax problems, invalid tokens");
    text("  Undefined variables   References to variables that were never declared");
    text("  Unknown methods       Calling methods that do not exist on a value");
    text("  Wrong argument count  Calling functions with the wrong number of arguments");
    text("  Type errors           Assigning incompatible types");
    section("WHAT IS NOT CAUGHT");
    text("  HTTP response values  Assertions on response status, body, or headers");
    text("  Network errors        Connection failures or timeouts");
    section("EXAMPLE");
    text("  def workflow.def --check");
    text("  # workflow.def: syntax ok");
    println!();
    text("  def broken.def --check");
    text("  # runtime error: unknown request method 'retry' at line 5 in 'broken.def'");
}

fn print_array() {
    text("ARRAY");
    section("DESCRIPTION");
    text("  Ordered collection of values with indexed access, iteration, and mutation.");
    text("  Arrays can hold mixed value types.");
    section("SYNTAX");
    text("  def items as array                         // empty");
    text("  def names as array(\"Marcelo\", \"Nicolas\")   // initialized");
    text("  names.push(\"Ana\")                          // append");
    text("  names[0]                                   // index access");
    text("  names.get(1)                               // method access");
    section("METHODS");
    text("  len()          Number of elements");
    text("  is_empty()     True when the array has no elements");
    text("  get(index)     Value at the given 0-based index");
    text("  push(value)    Append a value to the end");
    section("EXAMPLE");
    text("  def names as array(\"Marcelo\", \"Ana\", \"Nicolas\")");
    text("  names.push(\"Felicia\")");
    println!();
    text("  print(names.len())    // 4");
    text("  print(names[0])       // Marcelo");
    text("  print(names.get(1))   // Ana");
    println!();
    text("  for name in names (");
    text("    print(name)");
    text("  )");
    println!();
    text("  assert(names.len() == 4)");
    text("  assert(names[0] == \"Marcelo\")");
    text("  assert(names.is_empty() == false)");
}

fn print_assert() {
    text("ASSERT");
    section("DESCRIPTION");
    text("  Evaluates a boolean expression and aborts execution with an error if the");
    text("  result is false. Use assert to validate API responses and intermediate state.");
    text("  A failed assertion prints 'runtime error: assertion failed' and exits.");
    section("SYNTAX");
    text("  assert(boolean_expression)");
    section("EXAMPLE");
    text("  assert(1 + 2 == 3)");
    text("  assert(true)");
    text("  assert(not false)");
    println!();
    text("  def x as integer(10)");
    text("  assert(x > 0)");
    text("  assert(x == 10)");
    println!();
    text("  def res as response(request(GET).path(\"https://httpbingo.org/get\").do())");
    text("  assert(res.ok())");
    text("  assert(res.status() == 200)");
    text("  assert(res.body_contains(\"headers\"))");
}

fn print_body() {
    text("BODY  (aliases: jdef, tdef, json, text)");
    section("DESCRIPTION");
    text("  Sends a request body loaded from a template file. Two formats are supported:");
    text("  .jdef for JSON and .tdef for plain text. Both support {{variable}} placeholder");
    text("  substitution via with_var(). The Content-Type header is set automatically.");
    section(".JDEF FORMAT");
    text("  Raw JSON with {{variable}} placeholders. No comments.");
    println!();
    text("  {");
    text("    \"title\":  \"{{title}}\",");
    text("    \"userId\": {{user_id}}");
    text("  }");
    println!();
    text("  Note: bare {{user_id}} (no quotes) renders as a JSON number.");
    section(".TDEF FORMAT");
    text("  Plain text with {{variable}} placeholders.");
    text("  Lines starting with // or # are treated as comments and stripped before sending.");
    println!();
    text("  // request body");
    text("  name: {{name}}");
    text("  purpose: HTTP testing");
    section("SYNTAX");
    text("  request(POST)");
    text("    .path(url)");
    text("    .body_from(\"path/to/file.jdef\")");
    text("    .type(JSON)              // sets Content-Type: application/json");
    text("    .with_var(variable)      // registers a string variable for substitution");
    text("    .do()");
    println!();
    text("  Use .type(TEXT) with .tdef files.");
    section("EXAMPLE");
    text("  def title   as string(\"DefLang post\")");
    text("  def user_id as string(\"1\")");
    println!();
    text("  def res as response(");
    text("    request(POST)");
    text("      .path(\"https://jsonplaceholder.typicode.com/posts\")");
    text("      .body_from(\"jdef/post.jdef\")");
    text("      .type(JSON)");
    text("      .with_var(title)");
    text("      .with_var(user_id)");
    text("      .do()");
    text("  )");
    println!();
    text("  assert(res.status() == 201)");
    text("  print(\"created: {{res.describe_status()}}\")");
}

fn print_conditionals() {
    text("CONDITIONALS");
    section("DESCRIPTION");
    text("  if/else control flow. The condition must evaluate to a boolean.");
    text("  Blocks create a local scope: variables declared with def inside a block");
    text("  are not visible outside it. Assignments update the nearest enclosing");
    text("  variable — local if it exists in scope, global otherwise.");
    text("  The else branch is optional.");
    section("SYNTAX");
    text("  if condition (");
    text("    // then branch");
    text("  ) else (");
    text("    // else branch");
    text("  )");
    section("EXAMPLE");
    text("  def status  as integer(200)");
    text("  def message as string()");
    println!();
    text("  if status == 200 (");
    text("    message = \"ok\"");
    text("  ) else (");
    text("    message = \"unexpected\"");
    text("  )");
    println!();
    text("  assert(message == \"ok\")");
    println!();
    text("  // Conditional response handling");
    text("  def res as response(request(GET).path(\"https://httpbingo.org/get\").do())");
    println!();
    text("  if res.ok() (");
    text("    print(\"success: {{res.describe_status()}}\")");
    text("    assert(res.body_contains(\"headers\"))");
    text("  ) else (");
    text("    print(\"failure: {{res.describe_status()}}\")");
    text("  )");
}

fn print_datetime() {
    text("DATETIME");
    section("DESCRIPTION");
    text("  Captures the current system date and time at the moment the variable is");
    text("  declared. Individual parts can be read and updated. Setters return the");
    text("  updated datetime value. Useful for timestamping workflow runs.");
    section("FORMAT TOKENS");
    text("  hh     Hour (24h)");
    text("  mm     Minutes (when after hh) or month (when after dd)");
    text("  ss     Seconds");
    text("  dd     Day");
    text("  yy     Two-digit year");
    text("  yyyy   Four-digit year");
    section("METHODS");
    text("  format(mask)   Format as string using the tokens above");
    text("  hour()         Read or set the hour component");
    text("  minute()       Read or set the minute component");
    text("  second()       Read or set the second component");
    text("  day()          Read or set the day component");
    text("  month()        Read or set the month component");
    text("  year()         Read or set the year component");
    println!();
    text("  Calling any part method with an integer argument sets that component.");
    section("EXAMPLE");
    text("  def now as datetime");
    println!();
    text("  print(now.format(\"hh:mm:ss dd/mm/yyyy\"))");
    println!();
    text("  now.year(2026)");
    text("  now.month(1)");
    text("  now.day(1)");
    println!();
    text("  print(\"{{now.day()}}/{{now.month()}}/{{now.year()}}\")  // 1/1/2026");
    println!();
    text("  // Capture start and end timestamps");
    text("  def start_time  as datetime");
    text("  def started_at  as string(start_time.format(\"hh:mm:ss\"))");
    text("  // ... workflow ...");
    text("  def end_time    as datetime");
    text("  def finished_at as string(end_time.format(\"hh:mm:ss\"))");
    println!();
    text("  print(\"started:  {{started_at}}\")");
    text("  print(\"finished: {{finished_at}}\")");
}

fn print_delay() {
    text("DELAY");
    section("DESCRIPTION");
    text("  Pauses execution for a given number of milliseconds.");
    text("  The argument must be a non-negative integer.");
    text("  Useful for rate limiting between repeated API calls.");
    section("SYNTAX");
    text("  delay(milliseconds)");
    section("EXAMPLE");
    text("  print(\"Sending request...\")");
    text("  delay(500)");
    text("  def res as response(request(GET).path(\"https://httpbingo.org/get\").do())");
    text("  assert(res.ok())");
    println!();
    text("  // Pause between calls in a loop");
    text("  def ids  as array(\"1\", \"2\", \"3\")");
    text("  def base as string(\"https://api.example.com/posts/\")");
    println!();
    text("  for id in ids (");
    text("    def url as string(concat(base, id))");
    text("    def res as response(request(GET).path(url).do())");
    text("    print(\"{{id}}: {{res.describe_status()}}\")");
    text("    delay(200)");
    text("  )");
}

fn print_envvars() {
    text("ENVVARS");
    section("DESCRIPTION");
    text("  Loads environment variable defaults from a .edef file into the process");
    text("  environment. Values are then read with from_env_var() on a string.");
    println!();
    text("  Resolution order:");
    text("    1. If the variable is already set in the system environment, it is used");
    text("       as-is and the .edef value is ignored (a warning is printed to stderr).");
    text("    2. Otherwise the value from the .edef file is loaded and used.");
    text("    3. If the variable is set in neither place, execution aborts with:");
    text("       runtime error: environment variable 'X' is not set");
    section(".EDEF FORMAT");
    text("  One NAME=value per line. // and # lines are comments.");
    println!();
    text("  # API settings");
    text("  API_HOST=https://api.example.com");
    text("  API_KEY=dev-key-1234");
    text("  TIMEOUT=5000");
    section("SYNTAX");
    text("  def env  as envvars(\"edef/settings.edef\")");
    text("  def host as string().from_env_var(\"API_HOST\")");
    section("EXAMPLE");
    text("  def env     as envvars(\"edef/settings.edef\")");
    text("  def host    as string().from_env_var(\"API_HOST\")");
    text("  def api_key as string().from_env_var(\"API_KEY\")");
    println!();
    text("  // Override at runtime without modifying the file:");
    text("  // API_KEY=prod-secret def workflow.def");
    println!();
    text("  def res as response(");
    text("    request(GET)");
    text("      .path(concat(host, \"/users\"))");
    text("      .header(tuple(\"Authorization\", concat(\"Bearer \", api_key)))");
    text("      .do()");
    text("  )");
    println!();
    text("  assert(res.ok())");
    text("  print(\"status: {{res.describe_status()}}\")");
}

fn print_float() {
    text("FLOAT");
    section("DESCRIPTION");
    text("  64-bit floating-point numbers. Arithmetic operators +, -, *, /, % are");
    text("  supported. Mixed integer/float expressions produce float results.");
    section("SYNTAX");
    text("  def price as float(10.5)");
    text("  def price as float        // defaults to 0.0");
    section("OPERATORS");
    text("  +    Addition");
    text("  -    Subtraction");
    text("  *    Multiplication");
    text("  /    Division");
    text("  %    Modulo");
    text("  +=   Add and assign");
    text("  -=   Subtract and assign");
    section("EXAMPLE");
    text("  def price as float(19.99)");
    text("  def tax   as float(0.1)");
    text("  def total as float(price + price * tax)");
    println!();
    text("  print(\"price: {{price}}\")");
    text("  print(\"total: {{total}}\")");
    println!();
    text("  assert(total > price)");
    text("  assert(total < 30.0)");
    println!();
    text("  def discount   as float(0.15)");
    text("  def discounted as float(price - price * discount)");
    println!();
    text("  assert(discounted < price)");
    text("  assert(discounted > 0.0)");
}

fn print_function() {
    text("FUNCTION");
    section("DESCRIPTION");
    text("  Defines a named reusable function with typed parameters.");
    text("  Functions do not use 'return' — the last evaluated expression in the");
    text("  body is the return value. Functions can access module-level variables.");
    section("SYNTAX");
    text("  def name as function(param as type, ...) (");
    text("    // body");
    text("    expression  // return value");
    text("  )");
    section("EXAMPLE");
    text("  def sum as function(a as integer, b as integer) (");
    text("    a + b");
    text("  )");
    println!();
    text("  def n as integer(sum(10, 12))");
    text("  assert(n == 22)");
    println!();
    text("  // match as function body — maps a code to a label");
    text("  def http_label as function(code as integer) (");
    text("    match code (");
    text("      200 => \"200 OK\",");
    text("      201 => \"201 Created\",");
    text("      404 => \"404 Not Found\",");
    text("      _   => \"unknown\"");
    text("    )");
    text("  )");
    println!();
    text("  assert(http_label(200) == \"200 OK\")");
    text("  assert(http_label(999) == \"unknown\")");
    println!();
    text("  // Function accessing a module-level variable");
    text("  def base_url as string(\"https://api.example.com\")");
    println!();
    text("  def get_user as function(id as string) (");
    text("    request(GET)");
    text("      .path(concat(base_url, \"/users/\", id))");
    text("      .do()");
    text("  )");
    println!();
    text("  def res as response(get_user(\"1\"))");
    text("  assert(res.ok())");
}

fn print_headers() {
    text("HEADERS  (alias: hdef)");
    section("DESCRIPTION");
    text("  Set request headers inline using tuples, or load them from a .hdef file.");
    text("  Headers are case-insensitive. If the same header appears more than once,");
    text("  the last value wins — so a .header() call after .headers_from() overrides");
    text("  the file value for that specific header.");
    section(".HDEF FORMAT");
    text("  One 'Name: value' per line. // and # lines are comments.");
    text("  {{variable}} placeholders are substituted via with_var().");
    println!();
    text("  // common request headers");
    text("  Authorization: Bearer {{token}}");
    text("  Accept: application/json");
    text("  X-Client: deflang");
    section("SYNTAX");
    text("  // Inline");
    text("  request(GET)");
    text("    .header(tuple(\"Accept\", \"application/json\"))");
    text("    .header(tuple(\"Authorization\", \"Bearer token\"))");
    text("    .do()");
    println!();
    text("  // From file");
    text("  request(GET)");
    text("    .headers_from(\"hdef/common.hdef\")");
    text("    .with_var(token)");
    text("    .do()");
    println!();
    text("  // Override a file header with an inline value");
    text("  request(GET)");
    text("    .headers_from(\"hdef/common.hdef\")");
    text("    .header(tuple(\"Authorization\", \"Bearer override\"))");
    text("    .do()");
    section("EXAMPLE");
    text("  def token as string(\"my-api-key\")");
    println!();
    text("  def res as response(");
    text("    request(GET)");
    text("      .path(\"https://httpbingo.org/headers\")");
    text("      .header(tuple(\"Accept\", \"application/json\"))");
    text("      .header(tuple(\"Authorization\", concat(\"Bearer \", token)))");
    text("      .do()");
    text("  )");
    println!();
    text("  assert(res.ok())");
    text("  print(\"Authorization: {{res.header(\\\"authorization\\\")}}\")");
}

fn print_imported() {
    text("IMPORTED");
    section("DESCRIPTION");
    text("  Loads another .def file into its own isolated interpreter context and");
    text("  exposes its functions and variables as members of the binding name.");
    text("  The path is resolved relative to the importing file.");
    text("  The .def extension may be omitted.");
    println!();
    text("  Module-level variables can be read and assigned from outside.");
    text("  Functions inside a module have access to the module's own variables.");
    section("SYNTAX");
    text("  def mod as imported(\"path/to/module\")");
    println!();
    text("  mod.function_name(args)");
    text("  mod.variable_name");
    text("  mod.variable_name = new_value");
    section("EXAMPLE");
    text("  // math.def");
    text("  def add as function(a as integer, b as integer) (");
    text("    a + b");
    text("  )");
    text("  def multiplier as integer(2)");
    println!();
    text("  // main.def");
    text("  def math as imported(\"math\")");
    println!();
    text("  assert(math.add(10, 12) == 22)");
    println!();
    text("  math.multiplier = 3");
    text("  assert(math.multiplier == 3)");
    println!();
    text("  // API module pattern");
    text("  def api as imported(\"api\")");
    println!();
    text("  def res as response(api.get_user(\"1\"))");
    text("  assert(res.ok())");
    text("  print(\"{{api.base_url}} responded: {{res.describe_status()}}\")");
}

fn print_integer() {
    text("INTEGER");
    section("DESCRIPTION");
    text("  64-bit signed integer numbers. Arithmetic operators +, -, *, /, % are");
    text("  supported. Division of two integers produces a float.");
    section("SYNTAX");
    text("  def n as integer(10)");
    text("  def n as integer        // defaults to 0");
    section("OPERATORS");
    text("  +    Addition");
    text("  -    Subtraction");
    text("  *    Multiplication");
    text("  /    Division (always produces float)");
    text("  %    Modulo");
    text("  +=   Add and assign");
    text("  -=   Subtract and assign");
    section("COMPARISON");
    text("  ==   Equal          !=   Not equal");
    text("  >    Greater than   <    Less than");
    text("  >=   Greater or equal   <=   Less or equal");
    section("EXAMPLE");
    text("  def a as integer(10)");
    text("  a += 5");
    text("  a -= 3");
    println!();
    text("  def b as integer(a * 2)");
    text("  def c as integer(b % 7)");
    println!();
    text("  print(\"a = {{a}}\")   // 12");
    text("  print(\"b = {{b}}\")   // 24");
    text("  print(\"c = {{c}}\")   // 3");
    println!();
    text("  assert(a == 12)");
    text("  assert(b > a)");
    text("  assert(c >= 0)");
    text("  assert(c <= 6)");
}

fn print_match() {
    text("MATCH");
    section("DESCRIPTION");
    text("  Compares a value against literal patterns in order and evaluates the");
    text("  expression of the first matching arm. _ is the catch-all wildcard.");
    text("  match is an expression — it can initialize a variable, serve as a function");
    text("  body, or appear anywhere a value is expected.");
    section("SYNTAX");
    text("  match value (");
    text("    pattern => expression,");
    text("    pattern => expression,");
    text("    _       => expression");
    text("  )");
    section("EXAMPLE");
    text("  def status as integer(201)");
    println!();
    text("  def label as string(");
    text("    match status (");
    text("      200 => \"ok\",");
    text("      201 => \"created\",");
    text("      404 => \"not found\",");
    text("      _   => \"unexpected\"");
    text("    )");
    text("  )");
    println!();
    text("  assert(label == \"created\")");
    println!();
    text("  // match as a function body");
    text("  def describe as function(code as integer) (");
    text("    match code (");
    text("      200 => \"200 OK\",");
    text("      201 => \"201 Created\",");
    text("      404 => \"404 Not Found\",");
    text("      500 => \"500 Internal Server Error\",");
    text("      _   => \"unknown\"");
    text("    )");
    text("  )");
    println!();
    text("  def res as response(request(GET).path(\"https://httpbingo.org/get\").do())");
    text("  print(\"verdict: {{describe(res.status())}}\")");
}

fn print_query_string() {
    text("QUERY_STRING  (alias: qdef)");
    section("DESCRIPTION");
    text("  Appends URL query parameters to a request inline using tuples, or loads");
    text("  them from a .qdef file. If the same parameter name appears more than once,");
    text("  the last value wins.");
    section(".QDEF FORMAT");
    text("  One 'name: value' per line. // and # lines are comments.");
    text("  {{variable}} placeholders are substituted via with_var().");
    println!();
    text("  // search parameters");
    text("  search: {{search_term}}");
    text("  page: 1");
    text("  per_page: 20");
    section("SYNTAX");
    text("  // Inline");
    text("  request(GET)");
    text("    .path(url)");
    text("    .query_string(tuple(\"page\", \"1\"))");
    text("    .query_string(tuple(\"search\", \"deflang\"))");
    text("    .do()");
    println!();
    text("  // From file");
    text("  request(GET)");
    text("    .path(url)");
    text("    .query_string_from(\"qdef/params.qdef\")");
    text("    .with_var(search_term)");
    text("    .do()");
    section("EXAMPLE");
    text("  def search_term as string(\"deflang\")");
    println!();
    text("  def res as response(");
    text("    request(GET)");
    text("      .path(\"https://httpbingo.org/anything\")");
    text("      .query_string(tuple(\"search\", search_term))");
    text("      .query_string(tuple(\"page\", \"1\"))");
    text("      .do()");
    text("  )");
    println!();
    text("  assert(res.ok())");
    text("  assert(res.body_contains(\"deflang\"))");
    text("  print(\"status: {{res.describe_status()}}\")");
}

fn print_request() {
    text("REQUEST");
    section("DESCRIPTION");
    text("  Builds and executes an HTTP request using a fluent method chain.");
    text("  Call .do() to send the request — it returns a response value.");
    text("  Supports GET, POST, PUT, PATCH, DELETE, and any other HTTP method.");
    section("BUILDER METHODS");
    text("  .path(url)                    Set the request URL");
    text("  .header(tuple(name, value))   Add or replace a request header");
    text("  .headers_from(path)           Load headers from a .hdef file");
    text("  .query_string(tuple(k, v))    Append a query parameter");
    text("  .query_string_from(path)      Load query params from a .qdef file");
    text("  .body_from(path)              Load body from a .jdef or .tdef file");
    text("  .type(JSON)                   Set Content-Type: application/json");
    text("  .type(TEXT)                   Set Content-Type: text/plain");
    text("  .with_var(variable)           Register a string for template substitution");
    text("  .do()                         Send the request, return a response");
    section("EXAMPLE");
    text("  // GET with headers and query string");
    text("  def res as response(");
    text("    request(GET)");
    text("      .path(\"https://httpbingo.org/anything\")");
    text("      .header(tuple(\"Accept\", \"application/json\"))");
    text("      .query_string(tuple(\"page\", \"1\"))");
    text("      .do()");
    text("  )");
    println!();
    text("  assert(res.ok())");
    text("  print(\"{{res.describe_status()}} in {{res.duration()}}ms\")");
    println!();
    text("  // POST with JSON body");
    text("  def title as string(\"My post\")");
    println!();
    text("  def create as response(");
    text("    request(POST)");
    text("      .path(\"https://jsonplaceholder.typicode.com/posts\")");
    text("      .body_from(\"jdef/post.jdef\")");
    text("      .type(JSON)");
    text("      .with_var(title)");
    text("      .do()");
    text("  )");
    println!();
    text("  assert(create.status() == 201)");
    text("  print(\"created: {{create.describe_status()}}\")");
}

fn print_response() {
    text("RESPONSE");
    section("DESCRIPTION");
    text("  The value returned by request.do(). Provides access to the HTTP status,");
    text("  headers, and body. A default response (status 0, empty body) is the");
    text("  zero value when declared without an initializer.");
    section("METHODS");
    text("  status()              HTTP status code as integer");
    text("  ok()                  True when status is 2xx");
    text("  describe_status()     Human-readable label: \"200 OK\", \"404 Not Found\", ...");
    text("  duration()            Round-trip time in milliseconds");
    text("  size()                Response body size in bytes");
    text("  body()                Response body as string");
    text("  body_contains(text)   True when the body contains the given substring");
    text("  content_type()        Value of the Content-Type response header");
    text("  header(name)          Value of a specific header (case-insensitive)");
    text("  headers()             All headers as an array of tuple(name, value)");
    text("  expect(predicate)     Assert a condition; aborts with a readable error if false");
    section("EXAMPLE");
    text("  def res as response(");
    text("    request(GET)");
    text("      .path(\"https://httpbingo.org/anything\")");
    text("      .do()");
    text("  )");
    println!();
    text("  assert(res.ok())");
    text("  assert(res.status() == 200)");
    println!();
    text("  print(\"status:       {{res.describe_status()}}\")");
    text("  print(\"duration:     {{res.duration()}}ms\")");
    text("  print(\"size:         {{res.size()}} bytes\")");
    text("  print(\"content-type: {{res.content_type()}}\")");
    println!();
    text("  // Iterate all response headers");
    text("  for h in res.headers() (");
    text("    print(\"{{h.key()}}: {{h.value()}}\")");
    text("  )");
    println!();
    text("  // Read a specific header");
    text("  def ct as string(res.header(\"content-type\"))");
    text("  assert(ct != \"\")");
}

fn print_string() {
    text("STRING");
    section("DESCRIPTION");
    text("  UTF-8 text values. String concatenation uses the concat() builtin.");
    text("  There are no string arithmetic operators.");
    text("  String literals inside print() support {{expression}} interpolation.");
    section("SYNTAX");
    text("  def name as string(\"Marcelo\")");
    text("  def name as string()    // defaults to empty string \"\"");
    text("  def name as string      // same default");
    section("METHODS");
    text("  from_env_var(\"VAR\")   Read the value of an environment variable");
    section("BUILTINS");
    text("  concat(a, b, ...)   Join two or more strings");
    section("EXAMPLE");
    text("  def first as string(\"Marcelo\")");
    text("  def last  as string(\"Castellani\")");
    text("  def full  as string(concat(first, \" \", last))");
    println!();
    text("  print(full)                   // Marcelo Castellani");
    text("  print(\"Hello, {{full}}!\")       // Hello, Marcelo Castellani!");
    println!();
    text("  assert(full == \"Marcelo Castellani\")");
    text("  assert(full != \"\")");
    println!();
    text("  // Build a URL from parts");
    text("  def base as string(\"https://api.example.com\")");
    text("  def id   as string(\"42\")");
    text("  def url  as string(concat(base, \"/users/\", id))");
    println!();
    text("  // Reading from the system environment");
    text("  def home as string().from_env_var(\"HOME\")");
    text("  print(\"home: {{home}}\")");
}

fn print_expect() {
    text("EXPECT");
    section("DESCRIPTION");
    text("  Asserts a condition on a response value using readable field names.");
    text("  If the predicate is false, execution aborts with a descriptive error message");
    text("  that includes the predicate text and the current response values.");
    text("  expect() returns the response, so calls can be chained.");
    println!();
    text("  Available fields inside the predicate:");
    text("    status         HTTP status code (integer)");
    text("    ok             True when status is 2xx (boolean)");
    text("    duration       Round-trip time in milliseconds (integer)");
    text("    size           Response body size in bytes (integer)");
    text("    body           Response body (string)");
    text("    content_type   Value of the Content-Type header (string)");
    section("SYNTAX");
    text("  res.expect(predicate)");
    text("  res.expect(predicate).expect(predicate)   // chainable");
    section("EXAMPLE");
    text("  def res as response(");
    text("    request(GET)");
    text("      .path(\"https://httpbingo.org/anything\")");
    text("      .do()");
    text("  )");
    println!();
    text("  res.expect(ok)");
    text("  res.expect(status == 200)");
    text("  res.expect(duration < 5000)");
    println!();
    text("  // Chainable");
    text("  res.expect(ok).expect(status == 200).expect(duration < 5000)");
    println!();
    text("  // Error message when a predicate fails:");
    text("  // runtime error: expect(status == 201) failed: status=200, ok=true, duration=142ms");
    section("SEE ALSO");
    text("  def --help assert    Global assert() builtin");
    text("  def --help response  Full list of response methods");
}

fn print_tuple() {
    text("TUPLE");
    section("DESCRIPTION");
    text("  A key/value pair. The key must be a string; the value may be string,");
    text("  integer, float, or boolean. Tuples are the primary way to pass headers");
    text("  and query parameters to requests. Response headers are returned as tuples.");
    section("SYNTAX");
    text("  tuple(\"key\", value)");
    section("METHODS");
    text("  key()     The key string");
    text("  value()   The associated value");
    section("EXAMPLE");
    text("  def age as tuple(\"Age\", 48)");
    println!();
    text("  assert(age.key()   == \"Age\")");
    text("  assert(age.value() == 48)");
    println!();
    text("  // Tuples as request headers");
    text("  request(GET)");
    text("    .header(tuple(\"Accept\", \"application/json\"))");
    text("    .header(tuple(\"Authorization\", \"Bearer token\"))");
    text("    .do()");
    println!();
    text("  // Tuples as query parameters");
    text("  request(GET)");
    text("    .query_string(tuple(\"page\", \"1\"))");
    text("    .query_string(tuple(\"per_page\", \"20\"))");
    text("    .do()");
    println!();
    text("  // Response headers are an array of tuples");
    text("  def res as response(request(GET).path(\"https://httpbingo.org/get\").do())");
    println!();
    text("  for h in res.headers() (");
    text("    print(\"{{h.key()}}: {{h.value()}}\")");
    text("  )");
}
