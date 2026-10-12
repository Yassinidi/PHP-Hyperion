// سكربت بناء لربط مكتبات C القديمة أو LLVM
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
