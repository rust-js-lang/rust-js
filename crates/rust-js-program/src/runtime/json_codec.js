// A hand-written `Serialize` or `Deserialize`'s function, which gives a
// `Result`, as a codec, which throws its error as serde_json's reader and
// writer do theirs (ADR 0459).
function $jsonCodec(f) {
  return (...given) => {
    const result = f(...given);
    if (result.TAG === "Ok") return result._0;
    const e = result._0;
    throw e instanceof $JsonError ? e : new $JsonError(e.message, e.line, e.column);
  };
}
// A struct field's own reader, `deserialize_with`'s or `with`'s, as serde's
// derive calls it: never for a field that isn't there, which is `missing
// field` itself, an `Option`'s too, where a reader of no codec is given
// serde's `missing_field`, `None` of an `Option` (#2382). A default, where
// the field has one, is read before it's asked.
function $jsonPresent(read) {
  return (json) => {
    if (json instanceof $JsonMissing) throw $jsonError(`missing field \`${json.name}\``);
    return read(json);
  };
}
