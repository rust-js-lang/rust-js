// `d.deserialize_any(visitor)` and the other hints, of a hand-written
// `Visitor` (ADR 0459): what's there given to its `visit_*` of that kind,
// whose `Result` is what this gives; a number to its `u64`, `i64` or `f64` by
// its kind, as serde_json gives one; of a kind it has none for, serde's
// "invalid type", expecting what its `expecting` says. `expecting` is the
// visitor's, run only where an error's message is made, each time one is,
// as serde formats a `&dyn Expected` (#1544): what's expected is text made
// of it as it's written into the message.
function $jsonVisit(json, expecting, methods) {
  const expected = $jsonExpected(expecting);
  return $jsonRead(json, (reader) => reader.deserializeAny(expected, $jsonVisitor(expected, methods)));
}

// What a visitor expects, `expecting()`'s text made each time it's written.
function $jsonExpected(expecting) {
  return { toString: expecting };
}

// `deserialize_str(visitor)` and the other hints but `deserialize_any`: of a
// kind it has no method for, the reader's error, `deserializeHinted`'s: the
// text's of what it peeks at, before it's read, a sequence's or a map's
// where it starts; a value already read, the type it is (#236).
function $jsonVisitHint(json, expecting, methods) {
  const expected = $jsonExpected(expecting);
  const visitor = $jsonVisitor(expected, methods);
  return $jsonRead(json, (reader) => reader.deserializeHinted(expected, visitor));
}

// `deserialize_option(visitor)`: what's there given to its `visit_none`, its
// `visit_some`, the reader to read it, or its `visit_unit`, as each reader's
// `deserializeOption` says (#1597); of one it has none for, serde's
// "invalid type: Option value", as serde_json gives it.
function $jsonVisitOption(json, expecting, methods) {
  const expected = $jsonExpected(expecting);
  const visitor = $jsonVisitor(expected, methods);
  const invalid = () => $jsonError(`invalid type: Option value, expected ${expected}`);
  visitor.none = () => {
    if (!methods.none) throw invalid();
    return $jsonCodec(methods.none)();
  };
  visitor.some = (reader) => {
    if (!methods.some) throw invalid();
    return $jsonCodec(methods.some)(reader);
  };
  return $jsonRead(json, (reader) => reader.deserializeOption(expected, visitor));
}

// The reader's visitor of a hand-written one's methods.
function $jsonVisitor(expected, { unit, bool, u64, i64, f64, str, borrowedStr, string, seq, map }) {
  const invalid = (what) => $jsonError(`invalid type: ${what}, expected ${expected}`);
  const visitor = {};
  if (unit) visitor.unit = $jsonCodec(unit);
  if (bool) visitor.bool = $jsonCodec(bool);
  if (u64 || i64 || f64) {
    visitor.number = (n) => {
      const visit = n.kind === "u" ? u64 : n.kind === "i" ? i64 : f64;
      if (!visit) throw invalid($jsonUnexpectedNumber(n));
      // A `u64` or an `i64` is a BigInt, as rust-js holds one; an `f64` a number.
      return $jsonCodec(visit)(n.kind === "f" ? Number(n.value) : BigInt(n.value));
    };
  }
  // A string by the kind its reader gives (#1546): borrowed from the text
  // to `visit_borrowed_str`, owned to `visit_string`, each of whose serde
  // defaults is `visit_str`; one that's neither to `visit_str`.
  if (str || borrowedStr || string) {
    visitor.string = (s, kind) => {
      const visit = kind === "borrowed" ? (borrowedStr ?? str) : kind === "owned" ? (string ?? str) : str;
      if (!visit) throw invalid(`string ${$debugStr(s)}`);
      return $jsonCodec(visit)(s);
    };
  }
  // A sequence or a map, its access given, `SeqAccess` or `MapAccess`.
  if (seq) visitor.seq = $jsonCodec(seq);
  if (map) visitor.map = $jsonCodec(map);
  return visitor;
}
