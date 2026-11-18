fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .warnings(false)
        .include("vendor/include")
        .include("vendor/squirrel")
        .define("SQ_EXCLUDE_DEFAULT_MEMFUNCTIONS", None)
        .file("src/bridge.cpp");
    for file in [
        "sqapi",
        "sqbaselib",
        "sqclass",
        "sqcompiler",
        "sqdebug",
        "sqfuncstate",
        "sqlexer",
        "sqmem",
        "sqobject",
        "sqstate",
        "sqtable",
        "sqvm",
    ] {
        build.file(format!("vendor/squirrel/{file}.cpp"));
    }
    build.compile("crysirror_squirrel");
    println!("cargo:rerun-if-changed=src/bridge.cpp");
    println!("cargo:rerun-if-changed=vendor");
}
