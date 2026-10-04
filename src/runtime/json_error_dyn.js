// serde_json's error as a `dyn Error` (ADR 0141): shown as serde_json shows
// it, `{}` its message and `{:?}` `Error("..", line: 1, column: 1)`.
function $jsonErrorDyn() {
  return {
    Debug: () => ({ fmt: $debugJsonError }),
    Display: () => ({ fmt: $displayJsonError }),
    source: () => undefined,
  };
}
