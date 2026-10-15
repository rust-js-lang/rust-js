// A path's components, as Rust's `Path::components()` has them on Unix
// (ADR 0401): `/` first if it's absolute, `.` first if it starts with one,
// then each part between separators, but an empty one or `.`.
function $pathParts(path) {
  const pieces = path.split("/");
  const parts = path.startsWith("/") ? ["/"] : pieces[0] === "." ? ["."] : [];
  for (const piece of pieces) if (piece !== "" && piece !== ".") parts.push(piece);
  return parts;
}

// What tells paths apart, their components, `a/b` and `a//b/.` alike: one
// string each, which no other components make, as none but the first is
// empty, `/` or `.`.
function $pathKey(path) {
  return $pathParts(path).join("/");
}

// `a == b` of paths.
function $pathEq(a, b) {
  return $pathKey(a) === $pathKey(b);
}

// A path's `Ordering`: its components in turn, as Rust's `Component`
// orders them: the root, then `.`, then `..`, then a name, by code point.
// Only the first can be `/` or `.`.
function $pathCmp(a, b) {
  const x = $pathParts(a);
  const y = $pathParts(b);
  const rank = (part) => (part === "/" ? 0 : part === "." ? 1 : part === ".." ? 2 : 3);
  for (let i = 0; i < x.length && i < y.length; i++) {
    const order = rank(x[i]) - rank(y[i]) || $cmp(x[i], y[i]);
    if (order !== 0) return Math.sign(order);
  }
  return Math.sign(x.length - y.length);
}

// A map, or a set, of paths, each kept under its components, `$pathKey`'s:
// `a//b` finds `a/b`, which it gives back as it was put in.
class $PathMap extends $KeyMap {
  keyOf(path) {
    return $pathKey(path);
  }
}

class $PathSet extends $KeySet {
  keyOf(path) {
    return $pathKey(path);
  }
}
