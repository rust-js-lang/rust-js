// The class of a map or a set keyed by a tuple or an array, `(usize, T)`,
// made of its parts' classes, each as its part's key is found, where
// generic code gives a part's of a type only its caller knows (#1346): the
// caller gives each part's class, never the tuple's, so what a function
// asks of its callers stays its own parts', however deep a blanket impl
// makes them. Each class says how it finds its keys, its `way`: `Map` and
// `Set` by what JS finds them by, `$KeyMap` by value, `$keyMap(parts)`,
// `$PathMap` and `$CowMap` by what `parts` keeps each under, `$hashedMap`
// by its own `Hash` and `Eq`; a B-tree's by its `cmp`, its own `Ord`'s
// (`$ordMap`), one whose order can be seen (`$ordTree`), or std's
// (`$btreeMap` of a base). The tuple's, as std's derived `Hash`, `Eq` and
// `Ord` of a tuple find it: test/key-class-ways.txt holds each rule.
function $keyClassOf(set, sorted, parts) {
  const ways = parts.map($keyWayOf);
  if (!sorted) return $unsortedKeyClass(set, ways);
  const cmp = (a, b) => {
    for (let i = 0; i < ways.length; i++) {
      const order = ways[i].cmp(a[i], b[i]);
      if (order !== 0) return order;
    }
    return 0;
  };
  const kind = $keyClassSorted(ways.map((way) => way.sorted));
  if (kind === "tree") return [$ordTreeMap, $ordTreeSet][+set](cmp);
  if (kind === "own") return [$ordMap, $ordSet][+set](cmp);
  const base = $unsortedKeyClass(set, ways.map((way) => $keyWayOf(way.Base ?? [Map, Set][+set])));
  return [$btreeMap, $btreeSet][+set](cmp, base);
}

// How a class finds its keys: `Map`'s and `Set`'s by what JS finds them
// by, as `$key` finds a number or a string; another's, its `way`.
function $keyWayOf(Class) {
  return Class === Map || Class === Set ? { find: "value" } : Class.way;
}

// Of parts found as `ways` say, unsorted: by their own `Hash` and `Eq`
// where any part is, hashing each in its order and comparing each in its
// order, short of the first unequal, as a derived `Hash` and `PartialEq`
// of a tuple do; by what `parts` keeps each under where any part's is;
// else by value.
function $unsortedKeyClass(set, ways) {
  const kind = $keyClassUnsorted(ways.map((way) => way.find));
  const part = (way, value) => (way.find === "parts" ? way.parts(value) : value);
  if (kind === "hashed") {
    const KHash = {
      hash(key, state, H) {
        ways.forEach((way, i) => {
          if (way.find === "hashed") way.KHash.hash(key[i], state, H);
          else H.write(state, Array.from(new TextEncoder().encode($key(part(way, key[i])))));
        });
      },
    };
    const eq = (a, b) =>
      ways.every((way, i) => (way.find === "hashed" ? way.eq(a[i], b[i]) : $key(part(way, a[i])) === $key(part(way, b[i]))));
    return [$hashedMap, $hashedSet][+set](KHash, eq);
  }
  if (kind === "parts") return [$keyMap, $keySet][+set]((key) => ways.map((way, i) => part(way, key[i])));
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
