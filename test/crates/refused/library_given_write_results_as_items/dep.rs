pub fn oks<T, E, I: Iterator<Item = Result<T, E>>>(it: I) -> usize {
    it.filter(|r| r.is_ok()).count()
}
