// Rust's `Components` of a Unix path (ADR 0401): what's left of `path` to
// go over from its front and its back, as std's iterator keeps it, so what
// `parent()` gives is its slice of the path. A component is `/`, the root,
// `.` first, `..`, or a name, which is none of those. `front` and `back`
// are std's states: 1 before the body, 2 in it, 3 done.
function $components(path) {
  return { path, root: path.startsWith("/"), front: 1, back: 2, at: 0 };
}

// Whether what's left starts with a `.` that's a component. A rooted path's
// is asked only while its root is left, which a `.` isn't.
function $curDirFirst(c) {
  return c.path === "." || c.path.startsWith("./");
}

// How much of what's left is before its body: its root, or its first `.`.
function $beforeBody(c) {
  return c.front <= 1 ? Number(c.root) + Number($curDirFirst(c)) : 0;
}

// The text between separators that's a component: not an empty one, nor `.`.
function $partOf(text) {
  return text === "" || text === "." ? undefined : text;
}

// The text before the first separator left, and how much it takes.
function $frontText(c) {
  const at = c.path.indexOf("/");
  return at < 0 ? [c.path, c.path.length] : [c.path.slice(0, at), at + 1];
}

// The text after the last separator left in its body, and how much it takes.
function $backText(c) {
  const start = $beforeBody(c);
  const at = c.path.lastIndexOf("/");
  return at >= start ? [c.path.slice(at + 1), c.path.length - at] : [c.path.slice(start), c.path.length - start];
}

function $nextComponent(c) {
  while (c.front !== 3 && c.back !== 3 && c.front <= c.back) {
    if (c.front === 1) {
      c.front = 2;
      if (c.root || $curDirFirst(c)) {
        const part = c.root ? "/" : ".";
        c.path = c.path.slice(1);
        return part;
      }
    } else if (c.path.length > 0) {
      const [text, size] = $frontText(c);
      c.path = c.path.slice(size);
      if ($partOf(text) !== undefined) return text;
    } else {
      c.front = 3;
    }
  }
  return undefined;
}

// The last component left; `at`, where in the path it starts.
function $nextComponentBack(c) {
  while (c.front !== 3 && c.back !== 3 && c.front <= c.back) {
    if (c.back === 2 && c.path.length > $beforeBody(c)) {
      const [text, size] = $backText(c);
      c.path = c.path.slice(0, c.path.length - size);
      c.at = c.path.length + size - text.length;
      if ($partOf(text) !== undefined) return text;
    } else if (c.back === 2) {
      c.back = 1;
    } else {
      c.back = 3;
      if (c.root || $curDirFirst(c)) {
        const part = c.root ? "/" : ".";
        c.path = c.path.slice(0, -1);
        return part;
      }
    }
  }
  return undefined;
}

// What's left, as a path: without the separators and `.`s at either end
// of its body, as std's `as_path` trims them.
function $componentsPath(c) {
  const d = { ...c };
  while (d.front === 2 && d.path.length > 0) {
    const [text, size] = $frontText(d);
    if ($partOf(text) !== undefined) break;
    d.path = d.path.slice(size);
  }
  while (d.back === 2 && d.path.length > $beforeBody(d)) {
    const [text, size] = $backText(d);
    if ($partOf(text) !== undefined) break;
    d.path = d.path.slice(0, d.path.length - size);
  }
  return d.path;
}

// A path's components, in order.
function $pathParts(path) {
  const c = $components(path);
  const parts = [];
  for (let part = $nextComponent(c); part !== undefined; part = $nextComponent(c)) parts.push(part);
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

// Whether a component is a name: not the root, a first `.`, nor `..`.
function $isName(part) {
  return part !== undefined && part !== "/" && part !== "." && part !== "..";
}

// `p.parent()`: what's before its last component, `None` of the root or
// of nothing.
function $pathParent(path) {
  const c = $components(path);
  const last = $nextComponentBack(c);
  return last === undefined || last === "/" ? undefined : $componentsPath(c);
}

// `p.ancestors()`: it, its parent, and theirs, to the last.
function $pathAncestors(path) {
  const all = [];
  for (let p = path; p !== undefined; p = $pathParent(p)) all.push(p);
  return all;
}

// `p.file_name()`: its last component, if it's a name.
function $pathFileName(path) {
  const last = $nextComponentBack($components(path));
  return $isName(last) ? last : undefined;
}

// A file name's stem and extension, split at its last `.`, as std's
// `rsplit_file_at_dot`: `.bashrc` is a stem alone, and a name with no `.`
// an extension alone, which `file_stem()` reads as its stem.
function $splitAtDot(name) {
  const at = name.lastIndexOf(".");
  if (at === 0) return [name, undefined];
  return at < 0 ? [undefined, name] : [name.slice(0, at), name.slice(at + 1)];
}

function $pathFileStem(path) {
  const name = $pathFileName(path);
  if (name === undefined) return undefined;
  const [stem, extension] = $splitAtDot(name);
  return stem ?? extension;
}

function $pathExtension(path) {
  const name = $pathFileName(path);
  if (name === undefined) return undefined;
  const [stem, extension] = $splitAtDot(name);
  return stem === undefined ? undefined : extension;
}

// `base.join(path)`, and `buf.push(path)`'s new text: `path` itself if it's
// absolute, else after a separator, where `base` doesn't end in one.
function $pathJoin(base, path) {
  if (path.startsWith("/")) return path;
  return base !== "" && !base.endsWith("/") ? `${base}/${path}` : base + path;
}

// `buf.pop()`: its parent's text, and whether it had one.
function $pathPop(buf) {
  const parent = $pathParent(buf);
  return parent === undefined ? [buf, false] : [parent, true];
}

// `buf.set_file_name(name)`: its parent's, if it has a name, joined with `name`.
function $pathSetFileName(buf, name) {
  return $pathJoin($pathFileName(buf) === undefined ? buf : $pathParent(buf), name);
}

// `buf.set_extension(extension)`: its text to the end of its stem, and the
// extension after a `.`, and whether it had a name; a separator in the
// extension panics, as std's does.
function $pathSetExtension(buf, extension) {
  if (extension.includes("/")) {
    throw new Error(`extension cannot contain path separators: ${$debugStr(extension)}`);
  }
  const c = $components(buf);
  const name = $nextComponentBack(c);
  if (!$isName(name)) return [buf, false];
  const [stem, rest] = $splitAtDot(name);
  const kept = buf.slice(0, c.at + (stem ?? rest).length);
  return [extension === "" ? kept : `${kept}.${extension}`, true];
}

// `p.with_extension(extension)`: a copy given it, as `set_extension` gives
// it, which std's cut at its old extension's `.` comes to as well.
function $pathWithExtension(path, extension) {
  return $pathSetExtension(path, extension)[0];
}

// What's left of `c` after the components of `prefix`, each the same, as
// std's `iter_after`: from the front, `next`, or the back; `undefined` if
// it doesn't start with them.
function $componentsAfter(c, prefix, next) {
  for (;;) {
    const rest = { ...c };
    const part = next(rest);
    const want = next(prefix);
    if (want === undefined) return c;
    if (part !== want) return undefined;
    c = rest;
  }
}

// `p.strip_prefix(base)`: `Ok` of what's after `base`'s components, or
// `Err` of a `StripPrefixError`, which holds nothing.
function $pathStripPrefix(path, base) {
  const rest = $componentsAfter($components(path), $components(base), $nextComponent);
  return rest === undefined ? { TAG: "Err", _0: undefined } : { TAG: "Ok", _0: $componentsPath(rest) };
}

function $pathStartsWith(path, base) {
  return $componentsAfter($components(path), $components(base), $nextComponent) !== undefined;
}

function $pathEndsWith(path, child) {
  return $componentsAfter($components(path), $components(child), $nextComponentBack) !== undefined;
}
