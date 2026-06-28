fn main() {
    // 编译 Slint UI 文件
    slint_build::compile("src/app.slint").unwrap();

    // 打印构建信息
    println!("cargo:rerun-if-changed=src/app.slint");
    println!("cargo:rerun-if-changed=src/components/*.slint");
    println!("cargo:rerun-if-changed=src/views/*.slint");
}