// `cow.to_mut()` of a `Cow` of text or a number, which JS never changes in
// place: it owns what it borrowed, and a handle on it (ADR 0152).
function $cowMut(cow) {
  cow.TAG = "Owned";
  return {
    get value() {
      return cow._0;
    },
    set value(owned) {
      cow._0 = owned;
    },
  };
}
