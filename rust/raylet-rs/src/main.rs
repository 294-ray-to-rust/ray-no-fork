fn main() {
    if let Err(err) = raylet_rs::raylet_main() {
        eprintln!("raylet_main failed: {}", err);
        std::process::exit(1);
    }
}
