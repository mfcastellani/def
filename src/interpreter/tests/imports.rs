use super::*;

#[test]
fn imports_module_functions_and_variables() {
    let value = run_with_base_dir(
        "def math as imported(\"imports/math\")\n\
             math.variable = \"Marcelo\"\n\
             assert(math.add(10, 12) == 22 == (math.variable == \"Marcelo\"))",
        "examples/language",
    );
    assert_eq!(value, Value::Boolean(true));
}
