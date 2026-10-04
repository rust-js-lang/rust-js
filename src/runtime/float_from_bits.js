// `f64::from_bits(bits)` of a `u64`, or `f32::from_bits` of a `u32`.
function $floatFromBits(bits, size) {
  const view = new DataView(new ArrayBuffer(size));
  if (size === 8) {
    view.setBigUint64(0, bits);
    return view.getFloat64(0);
  }
  view.setUint32(0, bits);
  return view.getFloat32(0);
}
