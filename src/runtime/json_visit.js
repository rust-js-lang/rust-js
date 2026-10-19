// `d.deserialize_any(visitor)` and the other hints, of a hand-written
// `Visitor` (ADR 0459): what's there given to its `visit_*` of that kind,
// whose `Result` is what this gives; a number to its `u64`, `i64` or `f64` by
// its kind, as serde_json gives one; of a kind it has none for, serde's
// "invalid type", expecting what its `expecting` says.
function $jsonVisit(json, expected, { unit, bool, u64, i64, f64, str, borrowedStr }) {
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
  return $jsonRead(json, (reader) => reader.deserializeAny(expected, visitor));
}
