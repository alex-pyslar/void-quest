use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    void_quest::server::run().await
}
