// A union's field `key` read (ADR 0406): what it holds there, or under one
// of `alike`, its fields of the same type, whose bytes are the same. A
// field it doesn't hold read as another type's bytes, which JS has none
// of, is rust-js's error.
function $unionField(union, key, ...alike) {
  for (const field of [key, ...alike]) if (Object.hasOwn(union, field)) return union[field];
  const held = Object.keys(union)[0];
  throw new Error(
    `rust-js does not support reading a union's \`${key}\` while it holds \`${held}\`, which reads its bytes as another type's`,
  );
}

// A union's field `key` written: the one it holds now.
function $unionSet(union, key, value) {
  for (const field of Object.keys(union)) if (field !== key) delete union[field];
  union[key] = value;
}
