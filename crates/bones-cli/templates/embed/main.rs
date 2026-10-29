//! {{name}}'s own executable: the engine composed in code, loading the
//! extension `bones-cli build` puts in `.bones/extensions/`.
//!
//! Add native modules with `.module(...)`; see the bones-kernel README.

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let result = bones_kernel::Kernel::new()
        .window("{{name}}", 800, 600)
        .renderer()
        .ui()
        .extensions_dir(root.join(".bones/extensions"))
        .saves_dir(root.join(".bones/states"))
        .devtools_from_env("BONES_DEVTOOLS_PORT")
        .offscreen_from_env("BONES_OFFSCREEN")
        .run();
    if let Err(error) = result {
        eprintln!("fatal: {error}");
        std::process::exit(1);
    }
}
