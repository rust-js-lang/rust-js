// A handle on `object[key]`, a `&mut` of a number or a string a field or an
// item is: `object` reached once, where it's borrowed, as Rust reaches the
// place, and the same object while the borrow lasts (ADR 0099).
function $handleOn(object, key) {
  return {
    get value() {
      return object[key];
    },
    set value(value) {
      object[key] = value;
    },
  };
}
