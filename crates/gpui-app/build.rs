fn main() {
    println!("cargo:rerun-if-changed=assets/app-icons/butterpaper-icon.ico");

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app-icons/butterpaper-icon.ico");
        if let Err(err) = res.compile() {
            panic!("failed to compile Windows resources: {err}");
        }
    }
}
