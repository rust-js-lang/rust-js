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
