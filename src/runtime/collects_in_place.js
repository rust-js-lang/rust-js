// Whether std collects an iterator into a `Vec` of items `size` bytes and
// `align`-aligned in place, in its source's buffer, `source`, its items'
// `[size, align]`, as a type parameter's caller gives it (`IInPlace`), or
// `undefined` where it collects otherwise: as alloc's
// `in_place_collectible`, of items as aligned and no smaller, neither
// zero-sized. A collect a panic stops drops what it collected first then,
// what's left otherwise (#423, #672).
function $collectsInPlace(source, size, align) {
  return source !== undefined && source[0] > 0 && size > 0 && source[1] === align && source[0] >= size;
}
