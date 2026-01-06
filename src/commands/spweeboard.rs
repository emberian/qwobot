use super::{log_command_source, Context, Error};
use regex::Regex;
use spweeboard_core::inference::InferenceEngine;
use spweeboard_core::{PromptCompiler, SpweeboardEngine};
use std::sync::{Arc, LazyLock};
use tracing::{debug, info, instrument, trace};

/// Regex to match and remove <think>...</think> blocks (including partial/unclosed)
static THINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match complete <think>...</think> blocks or unclosed <think>... at end
    Regex::new(r"(?s)<think>.*?(</think>|$)").unwrap()
});

/// Strip thinking tags from model output
fn strip_thinking(output: &str) -> String {
    THINK_REGEX.replace_all(output, "").trim().to_string()
}

/// Adapter to use our LlmEngine with spweeboard
pub struct LlamaInference {
    llm: Arc<crate::llm::LlmEngine>,
}

impl LlamaInference {
    pub fn new(llm: Arc<crate::llm::LlmEngine>) -> Self {
        Self { llm }
    }
}

impl InferenceEngine for LlamaInference {
    type Error = crate::error::QwobotError;

    async fn generate(&self, prompt: &str) -> Result<String, Self::Error> {
        self.llm.generate(prompt).await
    }

    async fn stream(
        &self,
        prompt: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<String>, Self::Error> {
        // For now, just do non-streaming and send the full result
        let result = self.llm.generate(prompt).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            let _ = tx.send(result).await;
        });
        Ok(rx)
    }
}

/// Interpret an SPW expression using the spweeboard engine
#[poise::command(slash_command, prefix_command)]
#[instrument(skip(ctx), fields(expression = %expression), level = "info")]
pub async fn spweeboard(
    ctx: Context<'_>,
    #[description = "Ground context for interpretation (optional)"]
    ground: Option<String>,
    #[description = "SPW symbolic expression to interpret"]
    #[rest]
    expression: String,
) -> Result<(), Error> {
    log_command_source(&ctx, "spweeboard");
    info!(expression = %expression, ground = ?ground, "Processing spweeboard");
    ctx.defer().await?;

    // Create inference adapter
    debug!("Creating LlamaInference adapter");
    let inference = LlamaInference::new(ctx.data().llm.clone());

    // Get current prompt template from watcher
    let template = ctx.data().prompt_state.template().await;
    debug!(template_len = template.len(), "Using current prompt template");

    // Create spweeboard engine with the current template
    let compiler = PromptCompiler::with_template(template);
    debug!("Creating SpweeboardEngine with custom compiler");
    let mut engine = SpweeboardEngine::with_compiler(inference, compiler);

    // Load ground if provided
    if let Some(ground_text) = &ground {
        use spweeboard_core::ground::Ground;
        debug!(ground = %ground_text, "Loading ground context");
        let ground = Ground::natural("user", "User Ground", ground_text.as_str());
        engine.load_ground(ground);
    }

    // Push each character of the expression
    debug!(len = expression.len(), "Pushing expression characters");
    for c in expression.chars() {
        engine.push(c);
    }
    trace!("All characters pushed");

    // Generate interpretation
    info!("Starting SPW interpretation generation");
    let raw_result = engine.generate().await?;
    info!(result_len = raw_result.len(), "Generation complete");
    trace!(result = %raw_result, "Full result");

    // Strip thinking tags from output
    let result = strip_thinking(&raw_result);
    debug!(
        raw_len = raw_result.len(),
        stripped_len = result.len(),
        "Stripped thinking tags"
    );

    // Truncate if needed
    let mut message = result;
    if message.len() > 1950 {
        debug!("Truncating message");
        message.truncate(1900);
        message.push_str("...\n\n*[truncated]*");
    }

    ctx.say(message).await?;
    info!("Response sent");

    Ok(())
}
