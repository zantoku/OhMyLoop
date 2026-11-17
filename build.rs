fn main() {
    let root = "vendor/rubberband";
    println!("cargo:rerun-if-changed={root}");
    let mut build = cc::Build::new();
    // windows.h otherwise rewrites std::min/std::max in the vendored sources.
    // Keep the vendor tree intact and apply the platform contract at its boundary.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        build.define("NOMINMAX", None);
    }
    build
        .cpp(true)
        .std("c++17")
        .include(root)
        .file(format!("{root}/single/RubberBandSingle.cpp"))
        .warnings(false)
        .compile("rubberband");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-lib=framework=Accelerate");
    }
}
