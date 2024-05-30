mod config;
mod git;
mod rer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rer::Rer::parse().await?.run().await?;
    Ok(())
}
