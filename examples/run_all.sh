#!/usr/bin/env bash
set -u

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

skip_http=false
if [[ "${1:-}" == "--skip-http" ]]; then
  skip_http=true
fi

local_examples=(
  "examples/assertions/assert.def"
  "examples/language/conditionals.def"
  "examples/language/delay.def"
  "examples/language/imported-math.def"
  "examples/language/match.def"
  "examples/language/returned-value.def"
  "examples/language/fibonacci.def"
  "examples/types/array.def"
  "examples/types/boolean.def"
  "examples/types/datetime.def"
  "examples/types/float.def"
  "examples/types/integer.def"
  "examples/types/string.def"
  "examples/types/tuple.def"
  "examples/env/request_env.def"
  "examples/mocks/basic_mock.def"
  "examples/language/error_handling.def"
  "examples/language/loops.def"
  "examples/assertions/json_assertions.def"
  "examples/brazilian_docs/cpf.def"
  "examples/brazilian_docs/cnpj.def"
)

http_examples=(
  "examples/language/basic_request.def"
  "examples/headers/request_headers.def"
  "examples/query-string/request_query_string.def"
  "examples/body/request_body_json.def"
  "examples/body/request_body_text.def"
  "examples/jsonplaceholder/main.def"
)

examples=("${local_examples[@]}")
if [[ "$skip_http" == false ]]; then
  examples+=("${http_examples[@]}")
fi

failed=0

for example in "${examples[@]}"; do
  echo
  echo "==> $example"
  if ! cargo run --manifest-path "$REPO_ROOT/Cargo.toml" -- run "$REPO_ROOT/$example"; then
    echo "FAILED: $example"
    failed=1
  fi
done

if [[ "$failed" -ne 0 ]]; then
  echo
  echo "Some examples failed."
  exit 1
fi

echo
echo "All examples passed."
