fn main() {
    let mut build = cc::Build::new();
    build
        .file("vendor/sqlite3.c")
        .file("src/shim.c")
        .include("vendor")
        .define("SQLITE_DEFAULT_MEMSTATUS", "0")
        .define("SQLITE_ENABLE_COLUMN_METADATA", None)
        .define("SQLITE_ENABLE_DBSTAT_VTAB", None)
        .define("SQLITE_ENABLE_FTS3", None)
        .define("SQLITE_ENABLE_FTS3_PARENTHESIS", None)
        .define("SQLITE_ENABLE_FTS5", None)
        .define("SQLITE_ENABLE_GEOPOLY", None)
        .define("SQLITE_ENABLE_MATH_FUNCTIONS", None)
        .define("SQLITE_ENABLE_PREUPDATE_HOOK", None)
        .define("SQLITE_ENABLE_RBU", None)
        .define("SQLITE_ENABLE_RTREE", None)
        .define("SQLITE_ENABLE_SESSION", None)
        .define("SQLITE_THREADSAFE", "1");

    if !cfg!(target_os = "windows") {
        build.flag("-fvisibility=hidden");
    }

    build.compile("sqlite3");
    println!("cargo:rerun-if-changed=vendor/sqlite3.c");
    println!("cargo:rerun-if-changed=vendor/sqlite3.h");
    println!("cargo:rerun-if-changed=src/shim.c");
}
