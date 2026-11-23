// A trait's provided method called through `dictionary`, a type parameter's
// or a `dyn`'s, or of a crate's iterator its value, which carries its own
// (#2414): its impl's own where it writes one, `dictionary[key]`, given
// `entry`, what that entry takes; else the trait's default over `Self`,
// `byDefault`, its call, made only then, as what it converts may change what
// it's given, `Iterator.from` arming a `$Steps` to close. As Rust's dispatch
// picks.
function $provided(dictionary, key, entry, byDefault) {
  const written = dictionary[key];
  return written === undefined ? byDefault() : written.apply(undefined, entry);
}
