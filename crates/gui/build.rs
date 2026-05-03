fn main() {
    // tokens.slint is imported transitively by appwindow.slint; slint-build
    // resolves the import graph automatically.
    slint_build::compile("ui/appwindow.slint").unwrap();
    println!("cargo:rerun-if-changed=ui/tokens.slint");
}
