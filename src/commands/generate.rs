use super::{log_command_source, Context, Error};

/// Complete text from a prompt using the loaded LLM
#[poise::command(slash_command, prefix_command)]
pub async fn complete(
    ctx: Context<'_>,
    #[description = "The prompt to generate from"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    log_command_source(&ctx, "complete");

    // Defer response for long-running operations
    ctx.defer().await?;

    let response = ctx.data().llm.generate(&prompt).await?;

    // Format with prompt preview
    let prompt_preview = if prompt.len() > 100 {
        format!("{}...", &prompt[..100])
    } else {
        prompt
    };

    // Build message and truncate final result to stay under Discord's 2000 char limit
    let mut message = format!("**Prompt:** {}\n\n**Response:**\n{}", prompt_preview, response);
    if message.len() > 1950 {
        message.truncate(1900);
        message.push_str("...\n\n*[truncated]*");
    }

    ctx.say(message).await?;

    Ok(())
}
