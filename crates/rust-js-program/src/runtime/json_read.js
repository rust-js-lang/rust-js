// A serde protocol call whose `Result` a hand-written impl looks at: what
// `run` gives, `Ok`, or the backend's error it throws, `Err`; any other
// throw, a panic, goes on (#1545, ADR 0459). Of a call of a `writer`, an
// error takes it back to where the call began, `rewind`: the text's
// writer keeps what it wrote, as serde_json's keeps its bytes, and a
// `Value`'s drops what it built of the value it failed in, as serde_json
// builds each value apart and adds it only once it's made.
function $jsonTry(run, writer) {
  const mark = writer?.mark?.();
  try {
    return { TAG: "Ok", _0: run() };
  } catch (e) {
    if (!(e instanceof $JsonError)) throw e;
    writer?.rewind?.(mark);
    return { TAG: "Err", _0: e };
  }
}
// `T::deserialize(d)` in a hand-written `Deserialize`: what `read` reads
// from the reader, `Ok`, or the error it throws, `Err`, which `?` gives on
// with its place as serde_json found it (ADR 0459).
function $jsonRead(json, read) {
  return $jsonTry(() => read(json));
}
