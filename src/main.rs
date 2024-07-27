mod cli;
mod config;
mod git;
mod i18n;
mod rer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    rer::Rer::parse().await?.run().await?;
    Ok(())
}
