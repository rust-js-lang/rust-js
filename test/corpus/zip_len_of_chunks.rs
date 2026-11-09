// `len()` of a zip held, std's struct, of a slice's chunks, which are an
// array of them, `$iter` of it, whose length is known, as rand_core 0.9's
// `fill_via_chunks` asks of `dest.by_ref().zip(src.by_ref())`: the fewer of
// the two, as std's `Zip` hints it, its parts lent and stepped on after
// (#1100).
fn fill_via_chunks(src: &[u32], dest: &mut [u8]) -> (usize, usize) {
    let size = core::mem::size_of::<u32>();
    let mut dest = dest.chunks_exact_mut(size);
    let mut src = src.iter();
    let zipped = dest.by_ref().zip(src.by_ref());
    let num_chunks = zipped.len();
    zipped.for_each(|(dest, src)| dest.copy_from_slice(src.to_le_bytes().as_ref()));
    let byte_len = num_chunks * size;
    if let Some(dest) = dest.next() {
        let n = dest.len();
        dest.copy_from_slice(&src.next().map_or([0; 4], |s| s.to_le_bytes())[..n]);
    }
    (num_chunks, byte_len)
}

fn main() {
    let mut dest = [0u8; 10];
    println!("{:?} {dest:?}", fill_via_chunks(&[0x0403_0201, 0x0807_0605, 9], &mut dest));
    let mut dest = [0u8; 4];
    println!("{:?} {dest:?}", fill_via_chunks(&[7, 8], &mut dest));
    let mut dest = [0u8; 12];
    println!("{:?} {dest:?}", fill_via_chunks(&[1], &mut dest));

    let v = [1, 2, 3, 4, 5, 6, 7];
    let w = [10, 20, 30];
    let z = v.chunks(2).zip(w.iter());
    println!("{} {:?}", z.len(), z.size_hint());
    let z = v.windows(3).zip(v.chunks_exact(3));
    println!("{} {:?}", z.len(), z.collect::<Vec<_>>());
    let mut c = v.rchunks(3);
    c.next();
    let z = c.by_ref().zip(w.iter().skip(1));
    println!("{} {:?}", z.len(), c.next());
}
