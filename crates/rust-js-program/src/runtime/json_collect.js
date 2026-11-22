// `s.collect_seq(items)` of a hand-written `Serialize` (ADR 0459): each
// item, written by `write`, in an array, as it's stepped, what's left of
// them closed where a step throws, a panic (#1141).
function $jsonCollectSeq(json, items, write) {
  json.beginArray();
  $eachOf(items, (item) => {
    json.element();
    write(item, json);
  });
  json.endArray();
}

// `s.collect_map(entries)`: each entry, its key's text then its value
// written, in an object.
function $jsonCollectMap(json, entries, key, write) {
  json.beginObject();
  $eachOf(entries, ([k, v]) => {
    json.key(key(k));
    write(v, json);
  });
  json.endObject();
}
