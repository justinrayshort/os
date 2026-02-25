#[cfg(all(target_arch = "wasm32", feature = "hydrate"))]
fn main() {
    site::hydrate();
}

#[cfg(all(target_arch = "wasm32", feature = "csr", not(feature = "hydrate")))]
fn main() {
    site::mount();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!(
        "This binary is intended for Trunk/WASM. Run: trunk serve crates/site/index.html --features csr --open (bin: site_app)"
    );
}
