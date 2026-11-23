// A trait's provided method called through `dictionary`, a type parameter's
// or a `dyn`'s (#2414): its impl's own where it writes one, `dictionary[key]`,
// given `entry`, what that entry takes; else the trait's default over `Self`,
// `provided`, given `byDefault`, what it takes, as Rust's dispatch picks. Each
// list is names, dictionaries and drops, so building both runs nothing.
function $provided(dictionary, key, provided, entry, byDefault) {
  const written = dictionary[key];
  return written === undefined ? provided.apply(undefined, byDefault) : written.apply(undefined, entry);
}
