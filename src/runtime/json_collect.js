// `s.collect_seq(items)` of a hand-written `Serialize` (ADR 0459): each
// item, written by `write`, in an array.
function $jsonCollectSeq(json, items, write) {
  json.beginArray();
  for (const item of items) {
    json.element();
    write(item, json);
  }
  json.endArray();
}

// `s.collect_map(entries)`: each entry, its key's text then its value
// written, in an object.
function $jsonCollectMap(json, entries, key, write) {
  json.beginObject();
  for (const [k, v] of entries) {
    json.key(key(k));
    write(v, json);
  }
  json.endObject();
}
