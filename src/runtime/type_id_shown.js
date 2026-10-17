// `{:?}` of a `TypeId`, its hash, which rust-js doesn't keep: rust-js's
// error where it's shown, as only an assertion that fails may show one
// (ADR 0331).
function $typeIdShown() {
  throw new Error("rust-js does not support showing a `TypeId`'s hash, which it doesn't keep");
}
