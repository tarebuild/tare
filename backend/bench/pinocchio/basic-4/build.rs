fn main() {
    let path = std::path::Path::new("programs").join("basic-4").join("src").join("lib.rs");
    println!("cargo:rustc-env=REFERENCE_SOURCE={}", path.display());
    println!("cargo:rerun-if-changed=build.rs");
}
