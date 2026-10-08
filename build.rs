fn main() {
    let root = "vendor/rubberband";
    println!("cargo:rerun-if-changed={root}");
    cc::Build::new()
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
