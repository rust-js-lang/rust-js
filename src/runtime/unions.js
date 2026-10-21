// A union's field `key` read (ADR 0406): what it holds there, or under one
// of `alike`, its fields of the same type, whose bytes are the same; or,
// of a field of another plain type, its bytes read as `key`'s, by that
// field's decoder (ADR 0562). A field it doesn't hold that isn't plain
// data, whose bytes JS has none of, is rust-js's error.
function $unionField(union, key, alike = [], decoders = {}) {
  for (const field of [key, ...alike]) if (Object.hasOwn(union, field)) return union[field];
  const held = Object.keys(union)[0];
  // Plain data's, as the field asked reads its bytes (ADR 0562). One JS
  // holds as an object, which fills the union, is then the field held, so
  // a write into it is the union's.
  if (decoders[held] !== undefined) {
    const decoded = decoders[held](union[held]);
    if (typeof decoded === "object" && decoded !== null) $unionSet(union, key, decoded);
    return decoded;
  }
  throw new Error(
    `rust-js does not support reading a union's \`${key}\` while it holds \`${held}\`, which reads its bytes as another type's`,
  );
}

// A union's field `key` written: the one it holds now.
function $unionSet(union, key, value) {
  for (const field of Object.keys(union)) if (field !== key) delete union[field];
  union[key] = value;
}
