// The class of a map or a set keyed by a tuple, `(usize, T)`, made of its
// parts' classes, each as its part's key is found, where generic code
// gives a part's of a type only its caller knows (#1346): the caller gives
// each part's class, never the tuple's, so what a function asks of its
// callers stays its own parts', however deep a blanket impl makes them.
// Each class says how it finds its keys, its `way`: `Map` and `Set` by what
// JS finds them by, `$KeyMap` by value, `$keyMap(parts)`, `$PathMap` and
// `$CowMap` by what `parts` keeps each under, `$hashedMap` by its own
// `Hash` and `Eq`; a B-tree's by its `cmp`, its own `Ord`'s (`$ordMap`),
// one whose order can be seen (`$ordTree`), or std's (`$btreeMap` of a
// base). The tuple's, as std's derived `Hash`, `Eq` and `Ord` of a tuple
// find it: test/key-class-ways.txt holds each rule. Of a struct that
// derives what finds it, its fields', in their order, `fields` reading them
// of an object, `(key) => [key.id, key.tag]`; a tuple struct is its array
// already (#1360).
function $keyClassOf(set, sorted, parts, fields) {
  return $composedKeyClass(set, sorted, parts.map($keyWayOf), (i) => i, fields);
}

// The class of a map or a set keyed by an array, `[T; N]`, of its item's
// class, `Item`, given once whatever `N` (#1371): each item found as
// `Item` finds it, as std's `Hash`, `PartialEq` and `Ord` of an array, a
// slice's, ask each item's in turn, so it's a tuple's of parts all of one
// way. One class an item, `[T; 536870907]` took the compiler's memory.
function $arrayKeyClassOf(set, sorted, Item) {
  return $composedKeyClass(set, sorted, [$keyWayOf(Item)], () => 0);
}

// Of keys whose part `i`, of `parts(key)`, the key itself where it's an
// array, is found by `ways[index(i)]`: hashed, compared and ordered a part
// at a time, in order, the first unequal deciding.
function $composedKeyClass(set, sorted, ways, index, parts = (key) => key) {
  if (!sorted) return $unsortedKeyClass(set, ways, index, parts);
  const cmp = (x, y) => {
    const a = parts(x);
    const b = parts(y);
    for (let i = 0; i < a.length; i++) {
      const order = ways[index(i)].cmp(a[i], b[i]);
      if (order !== 0) return order;
    }
    return 0;
  };
  const kind = $keyClassSorted(ways.map((way) => way.sorted));
  if (kind === "tree") return [$ordTreeMap, $ordTreeSet][+set](cmp);
  if (kind === "own") return [$ordMap, $ordSet][+set](cmp);
  const base = $unsortedKeyClass(set, ways.map((way) => $keyWayOf(way.Base ?? [Map, Set][+set])), index, parts);
  return [$btreeMap, $btreeSet][+set](cmp, base);
}

// How a class finds its keys: `Map`'s and `Set`'s by what JS finds them
// by, as `$key` finds a number or a string; another's, its `way`.
function $keyWayOf(Class) {
  return Class === Map || Class === Set ? { find: "value" } : Class.way;
}

// Of parts found as `ways[index(i)]` says, unsorted: by their own `Hash`
// and `Eq` where any part is, hashing each in its order and comparing each
// in its order, short of the first unequal, as a derived `Hash` and
// `PartialEq` of a tuple do; by what `parts` keeps each under where any
// part's is; else by value.
function $unsortedKeyClass(set, ways, index, parts = (key) => key) {
  const kind = $keyClassUnsorted(ways.map((way) => way.find));
  const part = (way, value) => (way.find === "parts" ? way.parts(value) : value);
  if (kind === "hashed") {
    const KHash = {
      hash(whole, state, H) {
        const key = parts(whole);
        for (let i = 0; i < key.length; i++) {
          const way = ways[index(i)];
          if (way.find === "hashed") way.KHash.hash(key[i], state, H);
          else H.write(state, Array.from(new TextEncoder().encode($key(part(way, key[i])))));
        }
      },
    };
    const eq = (x, y) => {
      const a = parts(x);
      const b = parts(y);
      for (let i = 0; i < a.length; i++) {
        const way = ways[index(i)];
        if (!(way.find === "hashed" ? way.eq(a[i], b[i]) : $key(part(way, a[i])) === $key(part(way, b[i])))) return false;
      }
      return true;
    };
    return [$hashedMap, $hashedSet][+set](KHash, eq);
  }
  if (kind === "parts") return [$keyMap, $keySet][+set]((key) => Array.from(parts(key), (item, i) => part(ways[index(i)], item)));
  return [$KeyMap, $KeySet][+set];
}

// The rules of test/key-class-ways.txt: of the parts' ways, the tuple's.
function $keyClassUnsorted(finds) {
  if (finds.includes("hashed")) return "hashed";
  if (finds.includes("parts")) return "parts";
  return "value";
}

function $keyClassSorted(orders) {
  if (orders.includes("tree")) return "tree";
  if (orders.includes("own")) return "own";
  return "cmp";
}
