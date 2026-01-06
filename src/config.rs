use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub model_id: String,
    pub quantization: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub prefix: Option<String>,
    /// URL to fetch the SPW prompt template from
    #[serde(default = "default_prompt_url")]
    pub prompt_url: String,
    /// How often to check for prompt changes (in seconds)
    #[serde(default = "default_prompt_poll_interval")]
    pub prompt_poll_interval_secs: u64,
}

fn default_max_tokens() -> u32 {
    1024
}

fn default_temperature() -> f32 {
    0.7
}

fn default_prompt_url() -> String {
    "https://raw.githubusercontent.com/spwplace/spweeboard/refs/heads/dev/crates/spweeboard-core/src/compiler/prompt.txt".to_string()
}

fn default_prompt_poll_interval() -> u64 {
    60
}

impl AppConfig {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(
                config::Environment::with_prefix("QWOBOT")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }
}
