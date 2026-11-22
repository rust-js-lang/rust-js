// serde's `de::Unexpected`, as its `Display` shows it: `integer `3``,
// `string "x"`, `unit value` (ADR 0459).
function $jsonUnexpected(u) {
  if (typeof u === "string") {
    return {
      Unit: "unit value",
      Option: "Option value",
      NewtypeStruct: "newtype struct",
      Seq: "sequence",
      Map: "map",
      Enum: "enum",
      UnitVariant: "unit variant",
      NewtypeVariant: "newtype variant",
      TupleVariant: "tuple variant",
      StructVariant: "struct variant",
    }[u];
  }
  switch (u.TAG) {
    case "Bool":
      return `boolean \`${u._0}\``;
    case "Unsigned":
    case "Signed":
      return `integer \`${u._0}\``;
    case "Float": {
      // A float with a decimal point, as serde's `WithDecimalPoint` shows one.
      const shown = $displayF64(u._0);
      return `floating point \`${Number.isFinite(u._0) && !shown.includes(".") ? `${shown}.0` : shown}\``;
    }
    case "Char":
      return `character \`${u._0}\``;
    case "Str":
      return `string ${$debugStr(u._0)}`;
    case "Bytes":
      return "byte array";
    default:
      return u._0;
  }
}

// serde's `OneOf` of names given in this order: `` `a` ``, `` `a` or `b` ``,
// `` one of `a`, `b`, `c` ``; `what`'s `there are no variants` of none.
function $jsonUnknown(what, name, names) {
  if (names.length === 0) return $jsonError(`unknown ${what} \`${name}\`, there are no ${what}s`);
  const quoted = names.map((n) => `\`${n}\``);
  const expected = names.length === 1 ? quoted[0] : names.length === 2 ? `${quoted[0]} or ${quoted[1]}` : `one of ${quoted.join(", ")}`;
  return $jsonError(`unknown ${what} \`${name}\`, expected ${expected}`);
}
