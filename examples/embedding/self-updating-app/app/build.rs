//! Tells cargo that the version is an input.
//!
//! `main.rs` reads `DEMO_APP_VERSION` through `option_env!`, which cargo
//! cannot see. Without this, building 1.1.0 and then 1.0.0 from one source
//! tree silently reuses the first binary -- and the example would demonstrate
//! an update that never happened.
fn main() {
    println!("cargo:rerun-if-env-changed=DEMO_APP_VERSION");
}
