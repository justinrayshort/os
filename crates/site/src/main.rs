#[cfg(all(target_arch = "wasm32", feature = "hydrate"))]
fn main() {
    site::hydrate();
}

#[cfg(any(not(target_arch = "wasm32"), not(feature = "hydrate")))]
fn main() {
    eprintln!(
        "This binary is intended for Trunk/WASM. Run: trunk serve crates/site/index.html --features hydrate --open"
    );
}
