mod commands;
mod config;
mod error;
mod llm;
mod prompt_watcher;
mod stats;

use commands::Data;
use poise::serenity_prelude as serenity;
use prompt_watcher::PromptState;
use stats::CommandStats;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize logging - trace level for our crates
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("qwobot=trace".parse()?)
                .add_directive("spweeboard_core=info".parse()?),
        )
        .init();

    info!("Loading configuration...");
    let app_config = config::AppConfig::load()?;

    info!("Loading model {} {}...", app_config.model_id, app_config.quantization);
    let llm = llm::LlmEngine::new(
        &app_config.model_id,
        &app_config.quantization,
        app_config.max_tokens,
        app_config.temperature,
    )
    .await?;
    let llm = Arc::new(llm);
    info!("Model loaded!");

    // Initialize prompt state and start watcher
    let prompt_state = PromptState::new();
    let _prompt_watcher = prompt_watcher::spawn_watcher(
        app_config.prompt_url.clone(),
        Duration::from_secs(app_config.prompt_poll_interval_secs),
        prompt_state.clone(),
    );

    let prompt_url = app_config.prompt_url.clone();

    // Initialize command stats tracking
    let stats = CommandStats::new();

    let discord_token =
        std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN environment variable required");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::complete(),
                commands::model(),
                commands::spweeboard(),
                commands::prompt_refresh(),
                commands::model_switch(),
                commands::bot_status(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: app_config.prefix.clone(),
                ..Default::default()
            },
            pre_command: |ctx| {
                Box::pin(async move {
                    let cmd_name = ctx.command().name.clone();
                    ctx.data().stats.record_invocation(&cmd_name).await;
                    info!(command = %cmd_name, user = %ctx.author().id, "Command started");
                })
            },
            post_command: |ctx| {
                Box::pin(async move {
                    let cmd_name = ctx.command().name.clone();
                    // Note: We can't easily get duration here without more infrastructure,
                    // so we record with zero duration. The error handler records failures.
                    ctx.data().stats.record_success(&cmd_name, Duration::ZERO).await;
                    info!(command = %cmd_name, "Command completed successfully");
                })
            },
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                info!("Registering slash commands...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                info!("Bot is ready!");
                Ok(Data { llm, prompt_state, prompt_url, stats })
            })
        })
        .build();

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::ClientBuilder::new(&discord_token, intents)
        .framework(framework)
        .await?;

    // Get shard manager for graceful shutdown
    let shard_manager = client.shard_manager.clone();

    // Spawn Ctrl+C handler to set bot offline immediately
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        info!("Received Ctrl+C, shutting down gracefully...");
        shard_manager.shutdown_all().await;
    });

    info!("Starting bot...");
    client.start().await?;

    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Box<dyn std::error::Error + Send + Sync>>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            let cmd_name = ctx.command().name.clone();
            let user_id = ctx.author().id.to_string();
            let error_str = error.to_string();

            tracing::error!(command = %cmd_name, user = %user_id, error = %error_str, "Command failed");

            // Record failure in stats
            ctx.data().stats.record_failure(&cmd_name, &error_str, &user_id, Duration::ZERO).await;

            // Provide user-friendly error message
            let user_message = format_user_error(&error_str);
            let _ = ctx.say(format!(
                "**Command failed:** `/{}`\n\n{}\n\n*If this keeps happening, try `/bot-status` to check system health.*",
                cmd_name, user_message
            )).await;
        }
        poise::FrameworkError::Setup { error, .. } => {
            tracing::error!("Setup error: {}", error);
        }
        poise::FrameworkError::ArgumentParse { error, input, ctx, .. } => {
            warn!(error = %error, input = ?input, "Argument parse error");
            let _ = ctx.say(format!(
                "**Invalid argument:** {}\n\nPlease check the command syntax and try again.",
                error
            )).await;
        }
        poise::FrameworkError::CommandCheckFailed { error, ctx, .. } => {
            if let Some(e) = error {
                warn!(error = %e, "Command check failed");
                let _ = ctx.say(format!("**Permission denied:** {}", e)).await;
            }
        }
        poise::FrameworkError::CooldownHit { remaining_cooldown, ctx, .. } => {
            let _ = ctx.say(format!(
                "**Slow down!** Please wait {:.1} seconds before using this command again.",
                remaining_cooldown.as_secs_f32()
            )).await;
        }
        other => {
            if let Err(e) = poise::builtins::on_error(other).await {
                tracing::error!("Error handling error: {}", e);
            }
        }
    }
}

/// Formats an error message to be more user-friendly.
fn format_user_error(error: &str) -> String {
    // Map common errors to friendlier messages
    if error.contains("ModelLoad") {
        return "The AI model failed to load. The bot may need to be restarted.".to_string();
    }
    if error.contains("Inference") {
        return "The AI failed to generate a response. This might be a temporary issue - please try again.".to_string();
    }
    if error.contains("timeout") || error.contains("Timeout") {
        return "The request took too long and timed out. Try a shorter prompt or try again later.".to_string();
    }
    if error.contains("rate limit") || error.contains("RateLimit") {
        return "Too many requests. Please wait a moment before trying again.".to_string();
    }
    if error.contains("permission") || error.contains("Permission") {
        return "You don't have permission to use this command.".to_string();
    }

    // Default: show the actual error but truncated
    if error.len() > 200 {
        format!("{}...", &error[..200])
    } else {
        error.to_string()
    }
}
