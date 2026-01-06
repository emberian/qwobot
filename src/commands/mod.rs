mod bot_status;
mod generate;
mod model;
mod model_switch;
mod prompt_refresh;
mod spweeboard;

pub use bot_status::bot_status;
pub use generate::complete;
pub use model::model;
pub use model_switch::model_switch;
pub use prompt_refresh::prompt_refresh;
pub use spweeboard::spweeboard;

use crate::llm::LlmEngine;
use crate::prompt_watcher::PromptState;
use crate::stats::CommandStats;
use std::sync::Arc;
use tracing::info;

pub struct Data {
    pub llm: Arc<LlmEngine>,
    pub prompt_state: PromptState,
    pub prompt_url: String,
    pub stats: CommandStats,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Log information about the source of a slash/prefix command
pub fn log_command_source(ctx: &Context<'_>, command_name: &str) {
    let source = match ctx {
        poise::Context::Application(_) => "slash",
        poise::Context::Prefix(_) => "prefix",
    };

    let user = ctx.author();
    let guild = ctx.guild_id().map(|g| g.to_string()).unwrap_or_else(|| "DM".to_string());
    let channel = ctx.channel_id();

    info!(
        command = command_name,
        source = source,
        user_id = %user.id,
        user_name = %user.name,
        guild = %guild,
        channel = %channel,
        "Command invoked"
    );
}
