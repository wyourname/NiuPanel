use anyhow::Result;
mod app;
mod common;
mod modules;
mod startup;

#[tokio::main]
async fn main() -> Result<()> {
    if startup::commands::handle_commands().await? {
        return Ok(());
    }

    let _guard = startup::run().await?;
    Ok(())
}
