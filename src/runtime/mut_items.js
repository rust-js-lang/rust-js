// `v.iter_mut()` of numbers or strings, which JS can't change in place: a
// handle on each item, whose `value` reads and writes it (ADR 0152). Each
// knows its slot, so a raw pointer to what it's to is the slot's,
// `$ptrOfItem`'s (#1283).
const $itemArray = Symbol("rust-js item of");
const $itemIndex = Symbol("rust-js item at");

function $mutItems(v) {
  return Array.from(v, (_, i) => ({
    get value() {
      return v[i];
    },
    set value(item) {
      v[i] = item;
    },
    [$itemArray]: v,
    [$itemIndex]: i,
  }));
}

// A raw pointer to what an item `$mutItems` lent is to: its slot's, byteorder's
// `n as *const f32` of `for n in numbers` (#1283).
function $ptrOfItem(handle) {
  return $ptr(handle[$itemArray], handle[$itemIndex]);
}
