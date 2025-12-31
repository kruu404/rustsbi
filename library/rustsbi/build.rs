// 文件路径：./library/rustsbi/build.rs
use std::env;
use std::path::PathBuf;

fn main() {
    // 告诉 Cargo，只有当 entry.S 文件发生变化时，才重新运行此构建脚本
    println!("cargo:rerun-if-changed=src/entry.S");
    println!("cargo:rerun-if-changed=src/kernel/jump.S");

    // 获取输出目录，编译后的中间文件将放在这里
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // 使用 `cc` crate 来编译汇编文件
    cc::Build::new()
    .file("src/entry.S")
    .target("riscv64gc-unknown-none-elf")
    .flag("-march=rv64gc")        // 明确指定架构
    .flag("-mabi=lp64d")          // 明确指定使用双精度浮点ABI
    .flag("-D__riscv_float_abi_double")  // 定义双精度ABI宏
    .compile("entry");

    cc::Build::new()
        .file("src/kernel/jump.S")
        .target("riscv64gc-unknown-none-elf")
        .flag("-march=rv64gc")
        .flag("-mabi=lp64d")
        .flag("-D__riscv_float_abi_double")
        .compile("jump");

    // 指示 Cargo 在链接时，需要在 `OUT_DIR` 中查找我们生成的库
    println!("cargo:rustc-link-search={}", out_dir.display());
    // 指示 Cargo 链接名为 `entry` 的静态库
    println!("cargo:rustc-link-lib=static=entry");
    println!("cargo:rustc-link-lib=static=jump");
}