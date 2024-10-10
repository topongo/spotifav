use spotifav::do_toggle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    do_toggle().await
}

