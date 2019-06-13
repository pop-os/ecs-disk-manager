use std::{env, path::PathBuf};

use bindgen::Builder;
use pkg_config::Config;

fn main() {
    let cryptsetup = Config::new().probe("libcryptsetup").unwrap();

    println!("{:?}", cryptsetup);

    let cryptsetup_includes =
        cryptsetup.include_paths.into_iter().map(|path| format!("-I{}", path.display()));

    let cryptsetup_libs = cryptsetup.libs.into_iter().map(|lib| ["-l", lib.as_str()].concat());

    let bindings = Builder::default()
        .header("wrapper.h")
        .clang_args(cryptsetup_includes)
        .clang_args(cryptsetup_libs)
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out_path.join("bindings.rs")).expect("Couldn't write bindings!");
}
