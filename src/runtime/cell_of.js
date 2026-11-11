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

// A `&mut` to a place that holds a `&mut` to an object, `cell`, a box of
// it, as generic code has it where that `&mut` is its type parameter's
// value, `&mut W` of `W = &mut v`: its `value` the object's in-place cell,
// as a `W` given `&mut v` by value is, and a write of it puts the `&mut`
// that cell holds in the place (#1300).
function $heldCellOf(cell) {
  return {
    get value() {
      return $cellOf(cell.value);
    },
    set value(value) {
      cell.value = value.value;
    },
  };
}

// What generic code gives back of a `&mut W` whose `W` is a `&mut` to an
// object, `box`, its box of the object's in-place cell, as its caller has
// it: a box whose `value` is the object, and a write of it puts that
// object's cell in generic code's box (#1303).
function $heldObjectOf(box) {
  return {
    get value() {
      return box.value.value;
    },
    set value(value) {
      box.value = $cellOf(value);
    },
  };
}

// The dictionary of a `&mut` generic code holds as its box, where std passes
// its trait on to what it's to, `impl<T: Display> Display for &mut T`: what
// it's to's, each entry given what each box holds, `given`'s `1` of a
// box, `2` of a list of them (ADR 0454, #1414).
function $throughBox(dictionary, entries) {
  const through = { ...dictionary };
  for (const [name, given] of Object.entries(entries)) {
    const entry = dictionary[name];
    if (entry === undefined) continue;
    through[name] = (...args) =>
      entry(...args.map((arg, i) => (given[i] === 1 ? arg.value : given[i] === 2 ? arg.map((item) => item.value) : arg)));
  }
  return through;
}
