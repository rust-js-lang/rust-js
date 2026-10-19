// `T::deserialize(d)` in a hand-written `Deserialize`: what `read` reads
// from the reader, `Ok`, or the error it throws, `Err`, which `?` gives on
// with its place as serde_json found it (ADR 0459).
function $jsonRead(json, read) {
  try {
    return { TAG: "Ok", _0: read(json) };
  } catch (e) {
    if (e instanceof $JsonError) return { TAG: "Err", _0: e };
    throw e;
  }
}
