// A `&mut` of what a generic `Option`'s `Some` holds, given a handle on the
// option: it reads what's in the `Some`, and writes the option a `Some` of
// what it's given (ADR 0051).
function $someMut(option) {
  return {
    get value() {
      return $someValue(option.value);
    },
    set value(value) {
      option.value = $some(value);
    },
  };
}
