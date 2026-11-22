// `next_back()` of a JS iterator that knows its back, a `$iter` or a
// `$iterator` given its `nextBack`: Rust's `Option` of its last item.
function $nextBack(it) {
  const step = it.back();
  return step.done ? undefined : step.value;
}
