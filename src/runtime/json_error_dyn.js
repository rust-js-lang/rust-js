// serde_json's error, of `TypeId` `type`, as a `dyn Error` (ADR 0141):
// shown as serde_json shows it, `{}` its message and `{:?}` `Error("..",
// line: 1, column: 1)`; its `type_id` `type` (ADR 0331).
function $jsonErrorDyn(type) {
  return {
    Debug: () => ({ fmt: $debugJsonError }),
    Display: () => ({ fmt: $displayJsonError, fmtInto: $writeJsonError }),
    source: () => undefined,
    typeId: () => type,
  };
}
