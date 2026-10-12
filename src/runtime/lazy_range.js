
function* $lazyRange(start, end = Infinity) {
  for (let i = start; i < end; i++) {
    yield i;
  }
}
