use super::{log_command_source, Context, Error};
use crate::prompt_watcher::fetch_prompt;
use tracing::{info, warn};

/// Manually refresh the SPW prompt template from the remote URL
#[poise::command(slash_command, prefix_command, rename = "prompt-refresh")]
pub async fn prompt_refresh(ctx: Context<'_>) -> Result<(), Error> {
    log_command_source(&ctx, "prompt-refresh");
    ctx.defer().await?;

    let url = &ctx.data().prompt_url;
    info!(url = url, "Manually refreshing prompt template");

    match fetch_prompt(url).await {
        Ok(template) => {
            let updated = ctx.data().prompt_state.update_if_changed(template).await;
            if updated {
                info!("Prompt template updated from manual refresh");
                ctx.say("Prompt template refreshed successfully.").await?;
            } else {
                info!("Prompt template unchanged after manual refresh");
                ctx.say("Prompt template is already up to date.").await?;
            }
        }
        Err(e) => {
            warn!(error = %e, "Failed to fetch prompt template");
            ctx.say(format!("Failed to refresh prompt: {}", e)).await?;
        }
    }

    Ok(())
}
