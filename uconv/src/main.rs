//! uconv binary entry point. The real logic lives in [`uconv::run_main`].

#[tokio::main]
async fn main() -> std::process::ExitCode {
    uconv::run_main().await
}
