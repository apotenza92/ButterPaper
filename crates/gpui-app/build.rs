fn main() {
    // Make icon changes deterministic across both variants.
    println!("cargo:rerun-if-changed=assets/app-icons/butterpaper-icon.ico");
    println!("cargo:rerun-if-changed=assets/app-icons/butterpaper-icon-beta.ico");

    #[cfg(target_os = "windows")]
    {
        let is_beta = std::env::var_os("CARGO_FEATURE_BETA").is_some();
        let icon_path = if is_beta {
            "assets/app-icons/butterpaper-icon-beta.ico"
        } else {
            "assets/app-icons/butterpaper-icon.ico"
        };

        let mut res = winresource::WindowsResource::new();
        res.set_icon(icon_path);
        if let Err(err) = res.compile() {
            panic!("failed to compile Windows resources: {err}");
        }
    }
}
