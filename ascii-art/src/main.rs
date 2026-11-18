fn main() {
    std::process::exit(ascii_art::cli::run(std::env::args().skip(1).collect()));
}
