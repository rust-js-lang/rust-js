// `Some(x)` of a generic `T` (ADR 0051): `x`, unless it looks like `None`,
// as `undefined`, `null` or such a box does. Then it's a box one deeper,
// which keeps a `null` it holds, as JS tells it from `undefined`.
function $some(x) {
  if (x === null) return { $someNone: 0, $value: null };
  if (x === undefined) return { $someNone: 0 };
  if (typeof x === "object" && "$someNone" in x) return { ...x, $someNone: x.$someNone + 1 };
  return x;
}
