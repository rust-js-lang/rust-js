// std's B-tree, node for node (alloc/src/collections/btree, rustc 1.99;
// ADR 0404, amended; #926): the map of keys whose order can be seen, one
// that prints, counts, may panic or isn't a total order (`order_seen`,
// `order_total`). Which keys a search asks of, and in what order, is its
// nodes' shape, which insert's splits, remove's merges and steals, a bulk
// build's right border, `split_off`'s borders and `append`'s cursor leave:
// each is std's, so each search asks `cmp(key, held)` what std's asks.
//
// A node holds up to eleven entries, `[key, value]`, the one array
// wherever it moves, so a handle on its value holds; an internal one, at
// `height` above the leaves, an edge more, each a child, which knows its
// parent and its place there, as std's nodes do.
const $TREE_CAPACITY = 11;
const $TREE_MIN_LEN = 5;

function $treeNode(height) {
  return { entries: [], edges: height > 0 ? [] : undefined, height, parent: undefined, at: 0 };
}

// Each child of `node` from `from` on told where it is,
// `correct_childrens_parent_links`.
function $treeAdopt(node, from) {
  for (let i = from; i < node.edges.length; i++) {
    node.edges[i].parent = node;
    node.edges[i].at = i;
  }
}

// `find_key_index`: from `start`, `cmp(key, held)` of each entry in turn,
// to the first not less: `[i, true]` where it's equal, else the edge `i`
// it goes down.
function $treeIndex(node, key, cmp, start) {
  const entries = node.entries;
  for (let i = start; i < entries.length; i++) {
    const order = cmp(key, entries[i][0]);
    if (order === 0) return [i, true];
    if (order < 0) return [i, false];
  }
  return [entries.length, false];
}

// `splitpoint`: where a full node of an insert at edge `i` splits, and
// which half, and where in it, the insert goes.
function $treeSplitPoint(i) {
  if (i < 5) return [4, false, i];
  if (i === 5) return [5, false, i];
  if (i === 6) return [5, true, 0];
  return [6, true, i - 7];
}

// `insert_fit`, of a leaf, or of an internal node an edge too, right of
// the entry.
function $treeFit(node, i, entry, edge) {
  node.entries.splice(i, 0, entry);
  if (edge === undefined) return;
  node.edges.splice(i + 1, 0, edge);
  $treeAdopt(node, i + 1);
}

// `split` at the entry `middle`: the node left of it, the entry, and a
// new node of what's right of it.
function $treeSplit(node, middle) {
  const right = $treeNode(node.height);
  right.entries = node.entries.splice(middle + 1);
  const entry = node.entries.pop();
  if (node.height > 0) {
    right.edges = node.edges.splice(middle + 1);
    $treeAdopt(right, 0);
  }
  return { left: node, entry, right };
}

// `do_merge` of the children each side of the parent's entry `i`: the
// entry and the right one's put at the left one's end, the right one gone.
function $treeMerge(parent, i) {
  const left = parent.edges[i];
  const right = parent.edges[i + 1];
  const length = left.entries.length;
  left.entries = left.entries.concat(parent.entries.splice(i, 1), right.entries);
  parent.edges.splice(i + 1, 1);
  $treeAdopt(parent, i + 1);
  if (left.height > 0) {
    left.edges = left.edges.concat(right.edges);
    $treeAdopt(left, length + 1);
  }
  return left;
}

// Whether the children each side of the parent's entry `i` fit one node.
function $treeCanMerge(parent, i) {
  return parent.edges[i].entries.length + 1 + parent.edges[i + 1].entries.length <= $TREE_CAPACITY;
}

// `bulk_steal_left`: `count` entries of the left child through the
// parent's entry `i` to the right one's front, and their edges.
function $treeStealLeft(parent, i, count) {
  const left = parent.edges[i];
  const right = parent.edges[i + 1];
  const moved = left.entries.splice(left.entries.length - count);
  right.entries = moved.slice(1).concat([parent.entries[i]], right.entries);
  parent.entries[i] = moved[0];
  if (left.height > 0) {
    right.edges = left.edges.splice(left.edges.length - count).concat(right.edges);
    $treeAdopt(right, 0);
  }
}

// `bulk_steal_right`: the right child's first `count` through the
// parent's entry `i` to the left one's end, and their edges.
function $treeStealRight(parent, i, count) {
  const left = parent.edges[i];
  const right = parent.edges[i + 1];
  const moved = right.entries.splice(0, count);
  left.entries = left.entries.concat([parent.entries[i]], moved.slice(0, count - 1));
  parent.entries[i] = moved[count - 1];
  if (left.height > 0) {
    const edges = left.edges.length;
    left.edges = left.edges.concat(right.edges.splice(0, count));
    $treeAdopt(left, edges);
    $treeAdopt(right, 0);
  }
}

// The leaf edges at either end of `node`'s subtree.
function $treeFirstLeaf(node) {
  while (node.height > 0) node = node.edges[0];
  return node;
}

function $treeLastLeaf(node) {
  while (node.height > 0) node = node.edges[node.entries.length];
  return node;
}

// `next_kv` of the leaf edge `edge`: the entry right of it, `{ node, i }`,
// up from a node's end; `undefined` past the last.
function $treeNextKv({ node, i }) {
  while (i >= node.entries.length) {
    if (node.parent === undefined) return undefined;
    i = node.at;
    node = node.parent;
  }
  return { node, i };
}

// `next_leaf_edge` of the entry `kv`: the leaf edge right after it.
function $treeNextLeafEdge({ node, i }) {
  if (node.height === 0) return { node, i: i + 1 };
  return { node: $treeFirstLeaf(node.edges[i + 1]), i: 0 };
}

// `find_lower_bound_index` and `find_upper_bound_index` of a `SearchBound`
// (`{ TAG: "Included" | "Excluded", _0: key }`, or "AllIncluded" or
// "AllExcluded"): the edge it's at, and the bound below it.
function $treeLowerIndex(node, bound, cmp) {
  if (bound === "AllIncluded") return [0, bound];
  if (bound === "AllExcluded") return [node.entries.length, bound];
  const [i, there] = $treeIndex(node, bound._0, cmp, 0);
  if (!there) return [i, bound];
  return bound.TAG === "Included" ? [i, "AllExcluded"] : [i + 1, "AllIncluded"];
}

function $treeUpperIndex(node, bound, cmp, start) {
  if (bound === "AllIncluded") return [node.entries.length, bound];
  if (bound === "AllExcluded") return [start, bound];
  const [i, there] = $treeIndex(node, bound._0, cmp, start);
  if (!there) return [i, bound];
  return bound.TAG === "Included" ? [i + 1, "AllExcluded"] : [i, "AllIncluded"];
}

// A `Bound` as a `SearchBound`, `from_range`'s.
function $treeSearchBound(bound) {
  return bound === "Unbounded" ? "AllIncluded" : bound;
}

class $OrdTree extends $OrdMap {
  #root = $treeNode(0);
  #length = 0;
  // Its entries put in once its own fields are made, after `super`'s.
  constructor(cmp, entries) {
    super(cmp);
    if (entries) {
      for (const [key, value] of entries) {
        this.set(key, value);
      }
    }
  }
  made() {
    return this.constructor === $OrdTree ? new $OrdTree(this.cmp) : new this.constructor();
  }
  // `search_tree`: down from the root, each node searched from its first.
  found(key) {
    let node = this.#root;
    for (;;) {
      const [i, there] = $treeIndex(node, key, this.cmp, 0);
      if (there) return $treeFound(node, i);
      if (node.height === 0) return { there, node, i };
      node = node.edges[i];
    }
  }
  endFound(last) {
    if (this.#length === 0) return undefined;
    const leaf = last ? $treeLastLeaf(this.#root) : $treeFirstLeaf(this.#root);
    return $treeFound(leaf, last ? leaf.entries.length - 1 : 0);
  }
  putFound(found, key, value) {
    if (found.there) {
      found.entry[1] = value;
      return found.entry;
    }
    const entry = [key, value];
    this.#insert(found, entry);
    return entry;
  }
  deleteFound(found) {
    if (found.there) this.#remove(found);
  }
  replaceFound(found, key) {
    found.entry[0] = key;
  }
  deleteHeld(key) {
    const kv = this.#held(this.#root, key);
    if (kv !== undefined) this.#remove(kv);
  }
  // The entry `{ node, i }` whose key is `key` itself, in order.
  #held = (node, key) => {
    for (let i = 0; i <= node.entries.length; i++) {
      if (node.height > 0) {
        const kv = this.#held(node.edges[i], key);
        if (kv !== undefined) return kv;
      }
      if (i < node.entries.length && node.entries[i][0] === key) return { node, i };
    }
    return undefined;
  };
  held() {
    return $treeEntries(this.#root, []);
  }
  get size() {
    return this.#length;
  }
  clear() {
    this.#root = $treeNode(0);
    this.#length = 0;
  }
  // In `from`'s shape, a clone's, `clone_subtree`'s.
  inOrder(entries, from) {
    const made = Array.from(entries, ([key, value]) => [key, value]);
    let next = 0;
    const copy = (node) => {
      const copied = $treeNode(node.height);
      for (let i = 0; i <= node.entries.length; i++) {
        if (node.height > 0) copied.edges.push(copy(node.edges[i]));
        if (i < node.entries.length) copied.entries.push(made[next++]);
      }
      if (node.height > 0) $treeAdopt(copied, 0);
      return copied;
    };
    this.#root = copy(from.#root);
    this.#length = made.length;
    return this;
  }
  // `clone_subtree`'s making, in order: each entry made by `make`, node by
  // node. As one panics, what its node's frame made is dropped by `drop`,
  // in order; of a later edge's, first the frame's entry made before it,
  // its locals, by `held`, then what the frame made before that entry.
  madeEach(make, drop, held) {
    const dropMade = (made) => drop && (() => $dropItems(made, drop));
    const clone = (node) => {
      if (node.height === 0) {
        const made = [];
        $unwinds(() => {
          for (const entry of node.entries) made.push(make(entry));
        }, dropMade(made));
        return made;
      }
      let made = clone(node.edges[0]);
      for (let i = 0; i < node.entries.length; i++) {
        const entry = $unwinds(() => make(node.entries[i]), dropMade(made));
        const below = $unwinds(
          () => clone(node.edges[i + 1]),
          held && (() => held(entry)),
          dropMade(made),
        );
        made = made.concat([entry], below);
      }
      return made;
    };
    return clone(this.#root);
  }
  // `insert_recursing` at the leaf edge `{ node, i }`: split up to a new
  // root where each is full; where it went, `{ node, i }`.
  #insert = ({ node, i }, entry) => {
    let split;
    let at;
    if (node.entries.length < $TREE_CAPACITY) {
      $treeFit(node, i, entry);
      at = { node, i };
    } else {
      const [middle, right, index] = $treeSplitPoint(i);
      split = $treeSplit(node, middle);
      at = { node: right ? split.right : split.left, i: index };
      $treeFit(at.node, index, entry);
    }
    while (split !== undefined) {
      const parent = split.left.parent;
      if (parent === undefined) {
        const root = $treeNode(split.left.height + 1);
        root.entries.push(split.entry);
        root.edges.push(split.left, split.right);
        $treeAdopt(root, 0);
        this.#root = root;
        break;
      }
      const edge = split.left.at;
      if (parent.entries.length < $TREE_CAPACITY) {
        $treeFit(parent, edge, split.entry, split.right);
        break;
      }
      const [middle, right, index] = $treeSplitPoint(edge);
      const next = $treeSplit(parent, middle);
      $treeFit(right ? next.right : next.left, index, split.entry, split.right);
      split = next;
    }
    this.#length++;
    return at;
  };
  // `remove_kv_tracking` of the entry `{ node, i }`, and the root popped
  // where it's left empty: the leaf edge where it was, `{ node, i }`.
  #remove = ({ node, i }) => {
    let at;
    if (node.height === 0) {
      at = this.#removeLeaf(node, i);
    } else {
      // Of an internal node, the greatest left of it taken out of its
      // leaf and put in its place, found again past the hole.
      const leaf = $treeLastLeaf(node.edges[i]);
      const left = leaf.entries[leaf.entries.length - 1];
      const hole = this.#removeLeaf(leaf, leaf.entries.length - 1);
      const kv = $treeNextKv(hole);
      kv.node.entries[kv.i] = left;
      at = $treeNextLeafEdge(kv);
    }
    this.#length--;
    const root = this.#root;
    if (root.height > 0 && root.entries.length === 0) {
      this.#root = root.edges[0];
      this.#root.parent = undefined;
    }
    return at;
  };
  // `remove_leaf_kv`: a leaf left under five entries steals from or
  // merges with its left sibling, else its right; then its ancestors.
  #removeLeaf = (node, i) => {
    node.entries.splice(i, 1);
    if (node.entries.length >= $TREE_MIN_LEN || node.parent === undefined) return { node, i };
    const parent = node.parent;
    let at;
    if (node.at > 0) {
      const left = parent.edges[node.at - 1];
      const length = left.entries.length;
      if ($treeCanMerge(parent, node.at - 1)) {
        $treeMerge(parent, node.at - 1);
        at = { node: left, i: length + 1 + i };
      } else {
        $treeStealLeft(parent, node.at - 1, 1);
        at = { node, i: 1 + i };
      }
    } else {
      if ($treeCanMerge(parent, 0)) $treeMerge(parent, 0);
      else $treeStealRight(parent, 0, 1);
      at = { node, i };
    }
    $treeFixUp(at.node.parent);
    return at;
  };
  // `split_off`: down both sides of where `key` goes, each node's suffix
  // moved to the right tree's, then each tree's border fixed.
  splitOff(key) {
    const rest = this.made();
    if (this.#length === 0) return rest;
    const pillar = $treeNode(this.#root.height);
    for (let node = pillar; node.height > 0; node = node.edges[0]) {
      node.edges.push($treeNode(node.height - 1));
      $treeAdopt(node, 0);
    }
    let left = this.#root;
    let right = pillar;
    for (;;) {
      const [i] = $treeIndex(left, key, this.cmp, 0);
      right.entries = left.entries.splice(i);
      if (left.height === 0) break;
      right.edges = right.edges.concat(left.edges.splice(i + 1));
      $treeAdopt(right, 1);
      left = left.edges[i];
      right = right.edges[0];
    }
    this.#root = $treeFixBorder(this.#root, true);
    rest.#root = $treeFixBorder(pillar, false);
    rest.#length = $treeCount(rest.#root);
    this.#length -= rest.#length;
    return rest;
  }
  // `range`: the bounds checked by `eq` and `partialCmp`, `s == e` then
  // `s > e`, as std's are; then the edges where they part, each side down
  // to its leaf; what's between them, in order.
  range(start, end, name, eq, partialCmp, checked) {
    if (checked && start !== "Unbounded" && end !== "Unbounded") {
      if (start.TAG === "Excluded" && end.TAG === "Excluded" && eq(start._0, end._0)) {
        throw new Error(`range start and end are equal and excluded in ${name}`);
      }
      if (partialCmp(start._0, end._0) > 0) {
        throw new Error(`range start is greater than range end in ${name}`);
      }
    }
    let node = this.#root;
    let lower = $treeSearchBound(start);
    let upper = $treeSearchBound(end);
    for (;;) {
      const [low, lowerBelow] = $treeLowerIndex(node, lower, this.cmp);
      const [high, upperBelow] = $treeUpperIndex(node, upper, this.cmp, low);
      lower = lowerBelow;
      upper = upperBelow;
      if (low < high) {
        let front = { node, i: low };
        let back = { node, i: high };
        while (front.node.height > 0) {
          const below = front.node.edges[front.i];
          const [i, bound] = $treeLowerIndex(below, lower, this.cmp);
          front = { node: below, i };
          lower = bound;
          const under = back.node.edges[back.i];
          const [j, other] = $treeUpperIndex(under, upper, this.cmp, 0);
          back = { node: under, i: j };
          upper = other;
        }
        const found = [];
        for (let edge = front; edge.node !== back.node || edge.i !== back.i; ) {
          const kv = $treeNextKv(edge);
          found.push(kv.node.entries[kv.i]);
          edge = $treeNextLeafEdge(kv);
        }
        return found;
      }
      if (node.height === 0) return [];
      node = node.edges[low];
    }
  }
  // `lower_bound`: the leaf edge before the first key the bound holds.
  #lowerBound = (bound) => {
    let node = this.#root;
    for (;;) {
      const [i, below] = $treeLowerIndex(node, bound, this.cmp);
      if (node.height === 0) return { node, i };
      node = node.edges[i];
      bound = below;
    }
  };
  // `extract_if(range, f)`'s `ExtractIfInner`: from the start's lower
  // bound, each entry to the end asked by `partialCmp` as it's reached,
  // `<=` an included end or `<` an excluded one; each `f` keeps taken out.
  extractIf(start, end, partialCmp, take) {
    let edge = this.#lowerBound($treeSearchBound(start));
    const tree = this;
    return {
      next() {
        for (let kv = $treeNextKv(edge); kv !== undefined; kv = $treeNextKv(edge)) {
          const entry = kv.node.entries[kv.i];
          if (end !== "Unbounded") {
            const order = partialCmp(entry[0], end._0);
            if (!(end.TAG === "Included" ? order <= 0 : order < 0)) return undefined;
          }
          if (take(entry)) {
            edge = tree.#remove(kv);
            return entry;
          }
          edge = $treeNextLeafEdge(kv);
        }
        return undefined;
      },
    };
  }
  // `append`'s `merge` of `other`, emptied: of each of its entries in
  // order, a cursor of this one stepped to the first not less, `cmp(held,
  // given)`, and there it's put before; where they're equal, the one held
  // taken out and put back with `other`'s value, as `$merging` drops what
  // it replaces, and what's left as a comparison panics.
  append(other, drop, dropKey) {
    if (other.#length === 0) return;
    // Of an empty one, `other`'s tree, swapped in.
    if (this.#length === 0) {
      [this.#root, other.#root] = [other.#root, this.#root];
      [this.#length, other.#length] = [other.#length, 0];
      return;
    }
    const items = Array.from(other.held());
    other.clear();
    let edge;
    $merging(items, drop, dropKey, (entry, first, unwinding) => {
      // The first of `other`'s found by `lower_bound`, the cursor's start.
      if (first) edge = $unwinds(() => this.#lowerBound({ TAG: "Included", _0: entry[0] }), unwinding);
      for (;;) {
        const kv = $treeNextKv(edge);
        const held = kv && kv.node.entries[kv.i];
        const order = kv && $unwinds(() => this.cmp(held[0], entry[0]), unwinding);
        if (kv === undefined || order > 0) {
          const at = this.#insert(edge, entry);
          edge = { node: at.node, i: at.i + 1 };
          return undefined;
        }
        if (order === 0) {
          edge = this.#remove(kv);
          const putBack = () => {
            held[1] = entry[1];
            edge = this.#insert(edge, held);
          };
          return { held: [held[0], held[1]], putBack };
        }
        edge = $treeNextLeafEdge(kv);
      }
    });
  }
  // A bulk build's (`bulk_build_from_sorted_iter`): each entry of `sorted`
  // whose key `ne` says isn't the next one's pushed at the right border,
  // each kept dropped as `DedupSortedIter` drops it, its key then its value;
  // then the right border filled. As `ne` or a drop panics, the one asked
  // dropped, then those after the next, then the next, as its `Peekable`
  // drops them; what's pushed is leaked, as std's root is.
  pushSorted(sorted, ne, drop) {
    let leaf = this.#root;
    let i = 0;
    const next = () => {
      for (;;) {
        if (i >= sorted.length) return undefined;
        const entry = sorted[i++];
        if (i >= sorted.length) return entry;
        const after =
          drop && (() => $inTurn(() => $dropItems(sorted.slice(i + 1), drop), () => drop(sorted[i])));
        const apart = $unwinds(
          () => ne(entry[0], sorted[i][0]),
          drop && (() => drop(entry)),
          after,
        );
        if (apart) return entry;
        if (drop) $unwinds(() => drop(entry), after);
      }
    };
    for (let entry = next(); entry !== undefined; entry = next()) {
      if (leaf.entries.length < $TREE_CAPACITY) {
        leaf.entries.push(entry);
      } else {
        let open = leaf.parent;
        while (open !== undefined && open.entries.length >= $TREE_CAPACITY) open = open.parent;
        if (open === undefined) {
          const root = $treeNode(this.#root.height + 1);
          root.edges.push(this.#root);
          $treeAdopt(root, 0);
          this.#root = root;
          open = root;
        }
        let pillar = $treeNode(open.height - 1);
        const top = pillar;
        while (pillar.height > 0) {
          pillar.edges.push($treeNode(pillar.height - 1));
          $treeAdopt(pillar, 0);
          pillar = pillar.edges[0];
        }
        open.entries.push(entry);
        open.edges.push(top);
        $treeAdopt(open, open.edges.length - 1);
        leaf = $treeLastLeaf(open);
      }
      this.#length++;
    }
    // `fix_right_border_of_plentiful`.
    for (let node = this.#root; node.height > 0; ) {
      const last = node.entries.length - 1;
      const right = node.edges[last + 1];
      if (right.entries.length < $TREE_MIN_LEN) $treeStealLeft(node, last, $TREE_MIN_LEN - right.entries.length);
      node = right;
    }
    return this;
  }
}

// What `found` gives of the entry `i` of `node`.
function $treeFound(node, i) {
  const entry = node.entries[i];
  return { there: true, key: entry[0], value: entry[1], entry, node, i };
}

// Its entries in order, put in `into`.
function $treeEntries(node, into) {
  for (let i = 0; i <= node.entries.length; i++) {
    if (node.height > 0) $treeEntries(node.edges[i], into);
    if (i < node.entries.length) into.push(node.entries[i]);
  }
  return into;
}

function $treeCount(node) {
  let count = node.entries.length;
  if (node.height > 0) for (const edge of node.edges) count += $treeCount(edge);
  return count;
}

// `fix_node_and_affected_ancestors`: each node under five entries, up
// from `node`, merged with or stolen into from its left sibling, else its
// right, until one isn't.
function $treeFixUp(node) {
  while (node !== undefined && node.entries.length < $TREE_MIN_LEN) {
    const parent = node.parent;
    if (parent === undefined) return;
    const length = node.entries.length;
    if (node.at > 0) {
      if ($treeCanMerge(parent, node.at - 1)) {
        $treeMerge(parent, node.at - 1);
      } else {
        $treeStealLeft(parent, node.at - 1, $TREE_MIN_LEN - length);
        return;
      }
    } else if ($treeCanMerge(parent, 0)) {
      $treeMerge(parent, 0);
    } else {
      $treeStealRight(parent, 0, $TREE_MIN_LEN - length);
      return;
    }
    node = parent;
  }
}

// `fix_right_border`, or `fix_left_border` (`!right`), of a split's tree:
// empty roots popped, then down its border each child merged with its
// sibling or filled from it to six, then empty roots popped again.
function $treeFixBorder(root, right) {
  root = $treeFixTop(root);
  if (root.entries.length === 0) return root;
  for (let node = root; node.height > 0; ) {
    const i = right ? node.entries.length - 1 : 0;
    if ($treeCanMerge(node, i)) {
      node = $treeMerge(node, i);
      continue;
    }
    const child = node.edges[right ? i + 1 : i];
    const count = $TREE_MIN_LEN + 1 - child.entries.length;
    if (count > 0) {
      if (right) $treeStealLeft(node, i, count);
      else $treeStealRight(node, i, count);
    }
    node = child;
  }
  return $treeFixTop(root);
}

// `fix_top`: an empty internal root popped, again and again.
function $treeFixTop(root) {
  while (root.height > 0 && root.entries.length === 0) {
    root = root.edges[0];
    root.parent = undefined;
  }
  return root;
}

// A set of items whose order can be seen: an `$OrdTree` of each to itself.
class $OrdTreeSet extends $OrdSet {
  constructor(cmp, items) {
    super(cmp, items, $OrdTree);
  }
}

// The class of a B-tree of keys whose order can be seen, ordered by `cmp`,
// as `$ordMap`'s is made.
function $ordTreeMap(cmp) {
  return class extends $OrdTree {
    constructor(entries) {
      super(cmp, entries);
    }
  };
}

function $ordTreeSet(cmp) {
  return class extends $OrdTreeSet {
    constructor(items) {
      super(cmp, items);
    }
  };
}
