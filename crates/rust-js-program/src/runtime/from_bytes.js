// `T::from_be_bytes(bytes)` or `from_le_bytes`: the integer of `size`
// bytes, in either order, signed or not.
function $fromBytes(bytes, size, little, signed) {
  const view = new DataView(Uint8Array.from(bytes).buffer);
  if (size === 16) {
    const low = view.getBigUint64(little ? 0 : 8, little);
    const high = view.getBigUint64(little ? 8 : 0, little);
    const n = (high << 64n) | low;
    return signed ? BigInt.asIntN(128, n) : n;
  }
  if (size === 8) return signed ? view.getBigInt64(0, little) : view.getBigUint64(0, little);
  if (size === 4) return signed ? view.getInt32(0, little) : view.getUint32(0, little);
  if (size === 2) return signed ? view.getInt16(0, little) : view.getUint16(0, little);
  return signed ? view.getInt8(0) : view.getUint8(0);
}
