// Compiles the `rsmpi` C shim library.
extern crate gcc;
// Generates the Rust header for the C API.
extern crate bindgen;
// Finds out information about the MPI library
extern crate build_probe_mpi;
// Inspect version of rustc compiler
extern crate rustc_version;

use std::env;
use std::path::Path;

fn main() {
    // Use `mpicc` wrapper rather than the system C compiler.
    env::set_var("CC", "mpicc");
    // Build the `rsmpi` C shim library.
    gcc::compile_library("librsmpi.a", &["src/ffi/rsmpi.c"]);

    // Try to find an MPI library
    let lib = match build_probe_mpi::probe() {
        Ok(lib) => lib,
        Err(errs) => {
            println!("Could not find MPI library for various reasons:\n");
            for (i, err) in errs.iter().enumerate() {
                println!("Reason #{}:\n{}\n", i, err);
            }
            panic!();
        }
    };

    // Let `rustc` know about the library search directories.
    for dir in &lib.lib_paths {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }

    let mut builder = bindgen::builder();
    // Let `bindgen` know about libraries and search directories.
    for lib in &lib.libs { builder.link(lib.clone()); }
    for dir in &lib.lib_paths { builder.clang_arg(format!("-L{}", dir.display())); }
    for dir in &lib.include_paths { builder.clang_arg(format!("-I{}", dir.display())); }

    // Tell `bindgen` where to find system headers.
    if let Some(clang_include_dir) = bindgen::get_include_dir() {
        builder.clang_arg("-I");
        builder.clang_arg(clang_include_dir);
    }

    // Generate Rust bindings for the MPI C API.
    let bindings = builder
        .header("src/ffi/rsmpi.h")
        .emit_builtins()
        .generate()
        .unwrap();

    // Write the bindings to disk.
    let out_dir = env::var("OUT_DIR").expect("cargo did not set OUT_DIR");
    let out_file = Path::new(&out_dir).join("functions_and_types.rs");
    bindings
        .write_to_file(out_file)
        .unwrap();

    // Access to extern statics has to be marked unsafe after 1.13.0
    if rustc_version::version_matches(">=1.13.0") {
        println!("cargo:rustc-cfg=extern_statics_are_unsafe");
    }
}
