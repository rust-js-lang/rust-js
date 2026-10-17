// `Cell::from_mut` of a `&mut` to an object, an array's or a struct's: a
// cell whose `value` is the object, which a write changes in place, as
// writing through the `&mut` does (ADR 0153).
function $cellOf(object) {
  return {
    get value() {
      return object;
    },
    set value(value) {
      $assign(object, value);
    },
  };
}
