use super::{log_command_source, Context, Error};

/// Display information about the loaded model
#[poise::command(slash_command, prefix_command)]
pub async fn model(ctx: Context<'_>) -> Result<(), Error> {
    log_command_source(&ctx, "model");

    let info = ctx.data().llm.model_info();

    ctx.say(format!("**Model Information**\n{}", info)).await?;

    Ok(())
}
