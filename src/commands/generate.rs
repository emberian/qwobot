use super::{log_command_source, Context, Error};
use regex::Regex;
use std::sync::LazyLock;

/// Regex to match and remove <think>...</think> blocks (including partial/unclosed)
static THINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<think>.*?(</think>|$)").unwrap()
});

/// Strip thinking tags from model output
fn strip_thinking(output: &str) -> String {
    THINK_REGEX.replace_all(output, "").trim().to_string()
}

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

    let raw_response = ctx.data().llm.generate(&prompt).await?;
    let response = strip_thinking(&raw_response);

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
