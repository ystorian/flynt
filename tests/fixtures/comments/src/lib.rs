/// Documents the helper, so this must not count as a usage:
/// - `loc("ghost", &lang)`
//! loc("inner-ghost", &lang)
// loc("line-ghost", &lang)
pub fn real() -> String {
	// The word boundary keeps these two out.
	let _ = alloc("not-a-key");
	let _ = realloc("also-not");
	loc("real-key", &lang)
}
