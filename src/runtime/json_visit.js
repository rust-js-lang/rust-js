// `d.deserialize_any(visitor)` and the other hints, of a hand-written
// `Visitor` (ADR 0459): what's there given to its `visit_*` of that kind,
// whose `Result` is what this gives; a number to its `u64`, `i64` or `f64` by
// its kind, as serde_json gives one; of a kind it has none for, serde's
// "invalid type", expecting what its `expecting` says.
function $jsonVisit(json, expected, methods) {
  return $jsonRead(json, (reader) => reader.deserializeAny(expected, $jsonVisitor(expected, methods)));
}

// `deserialize_str(visitor)` and the other hints but `deserialize_any`: of a
// kind it has no method for, serde_json's error of what it peeks at, before
// it's read, a sequence's or a map's where it starts.
function $jsonVisitHint(json, expected, methods) {
  const visitor = $jsonVisitor(expected, methods);
  return $jsonRead(json, (reader) => {
    const peek = reader.parseWhitespace();
    const kind =
      peek === 110
        ? "unit"
        : peek === 116 || peek === 102
          ? "bool"
          : peek === 45 || (peek >= 48 && peek <= 57)
            ? "number"
            : peek === 34
              ? "string"
              : peek === 91
                ? "seq"
                : peek === 123
                  ? "map"
                  : undefined;
    if (kind !== undefined && !visitor[kind]) throw reader.peekInvalidType(expected);
    return reader.deserializeAny(expected, visitor);
  });
}

// The reader's visitor of a hand-written one's methods.
function $jsonVisitor(expected, { unit, bool, u64, i64, f64, str, borrowedStr, seq, map }) {
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
  if (str || borrowedStr) {
    visitor.string = (s, borrowed) => {
      const visit = borrowed ? (borrowedStr ?? str) : str;
      if (!visit) throw invalid(`string ${$debugStr(s)}`);
      return $jsonCodec(visit)(s);
    };
  }
  // A sequence or a map, its access given, `SeqAccess` or `MapAccess`.
  if (seq) visitor.seq = $jsonCodec(seq);
  if (map) visitor.map = $jsonCodec(map);
  return visitor;
}
