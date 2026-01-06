use super::{log_command_source, Context, Error};
use tracing::info;

/// View bot status, command statistics, and recent failures
#[poise::command(slash_command, prefix_command, rename = "bot-status")]
pub async fn bot_status(ctx: Context<'_>) -> Result<(), Error> {
    log_command_source(&ctx, "bot-status");

    let summary = ctx.data().stats.summary().await;
    let model_id = ctx.data().llm.current_model_id();
    let model_quant = ctx.data().llm.current_quantization();

    // Build status message
    let mut msg = String::new();

    // Header
    msg.push_str("**Bot Status Dashboard**\n\n");

    // System info
    msg.push_str("**System**\n");
    msg.push_str(&format!("Uptime: `{}`\n", summary.uptime_display()));
    msg.push_str(&format!("Model: `{} ({})`\n", model_id, model_quant));
    msg.push_str(&format!("Model Info: {}\n\n", ctx.data().llm.model_info()));

    // Overall stats
    msg.push_str("**Command Statistics**\n");
    msg.push_str(&format!(
        "Total: {} | Success: {} | Failed: {} | Rate: {:.1}%\n\n",
        summary.total_invocations,
        summary.total_successes,
        summary.total_failures,
        summary.success_rate()
    ));

    // Per-command breakdown (top 5)
    if !summary.commands.is_empty() {
        msg.push_str("**Per-Command Breakdown**\n```\n");
        msg.push_str(&format!(
            "{:<15} {:>6} {:>6} {:>6} {:>8}\n",
            "Command", "Calls", "OK", "Fail", "Avg(ms)"
        ));
        msg.push_str(&format!("{}\n", "-".repeat(45)));

        for cmd in summary.commands.iter().take(5) {
            msg.push_str(&format!(
                "{:<15} {:>6} {:>6} {:>6} {:>8}\n",
                truncate(&cmd.name, 15),
                cmd.invocations,
                cmd.successes,
                cmd.failures,
                cmd.avg_latency_ms
            ));
        }
        msg.push_str("```\n");
    }

    // Recent failures
    if !summary.recent_failures.is_empty() {
        msg.push_str("**Recent Failures** (last 5)\n```\n");

        for failure in summary.recent_failures.iter().take(5) {
            let ago = failure
                .timestamp
                .elapsed()
                .map(|d| format_duration(d))
                .unwrap_or_else(|_| "?".to_string());

            msg.push_str(&format!(
                "[{}] /{}: {}\n",
                ago,
                failure.command,
                truncate(&failure.error, 40)
            ));
        }
        msg.push_str("```\n");
    } else {
        msg.push_str("**Recent Failures:** None\n");
    }

    // Truncate if too long for Discord
    if msg.len() > 1900 {
        msg.truncate(1850);
        msg.push_str("\n\n*[truncated]*");
    }

    ctx.say(msg).await?;
    info!("Bot status displayed");

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}
