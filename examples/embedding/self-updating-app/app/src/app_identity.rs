use bones_kernel::upgrade::AppIdentity;

pub fn app_identity() -> AppIdentity {
    AppIdentity {
        install_dir_name: option_env!("UPGRADER_INSTALL_DIR_NAME").unwrap_or(".demo-app"),
        install_env_var: option_env!("UPGRADER_INSTALL_ENV_VAR").unwrap_or("DEMO_APP_INSTALL_DIR"),
        launcher_stem: option_env!("UPGRADER_LAUNCHER_STEM").unwrap_or("demo"),
        app_stem: option_env!("UPGRADER_APP_STEM").unwrap_or("demo-app"),
    }
}
