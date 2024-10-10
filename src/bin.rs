
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sp = spotifav::get_client().await?;
    spotifav::do_toggle(&sp).await?;

    Ok(())
}

