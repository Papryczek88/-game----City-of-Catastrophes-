use std::{env, path::PathBuf, process::Command};

fn run(cmd: &mut Command, what: &str) {
    let status = cmd.status().unwrap_or_else(|e| panic!("{what}: {e}"));
    if !status.success() { panic!("{what} failed with status {status}"); }
}

fn main() {
    println!("cargo:rerun-if-changed=../ASM/income_sysv.asm");
    println!("cargo:rerun-if-changed=../ASM/income_win64.asm");
    if env::var_os("CARGO_FEATURE_ASM").is_none() { return; }

    let target = env::var("TARGET").expect("TARGET missing");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR missing"));
    let obj = out.join("coc_asm.obj");
    let lib = out.join(if target.contains("msvc") { "cocasm.lib" } else { "libcocasm.a" });
    let source = if target.contains("windows") { "../ASM/income_win64.asm" } else { "../ASM/income_sysv.asm" };

    run(Command::new("nasm").arg("-f").arg(if target.contains("windows") { "win64" } else { "elf64" }).arg(source).arg("-o").arg(&obj), "NASM assembly");
    if target.contains("msvc") {
        run(Command::new("lib.exe").arg(format!("/OUT:{}", lib.display())).arg(&obj), "MSVC librarian");
    } else {
        let ar = env::var("AR").unwrap_or_else(|_| "ar".into());
        run(Command::new(ar).arg("crus").arg(&lib).arg(&obj), "archiver");
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=cocasm");
}
