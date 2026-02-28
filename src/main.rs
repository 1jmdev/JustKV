use valkey::run;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("server error: {err}");
        std::process::exit(1);
    }
}
