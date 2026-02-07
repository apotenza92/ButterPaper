fn main() {
    if let Err(error) = butterpaper_cli::run(std::env::args_os()) {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
