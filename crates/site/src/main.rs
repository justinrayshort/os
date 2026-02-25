#[cfg(all(target_arch = "wasm32", feature = "csr"))]
fn main() {
    site::mount();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!(
        "This binary is intended for Trunk/WASM (CSR). Run: trunk serve crates/site/index.html --features csr --open (bin: site_app)"
    );
}
