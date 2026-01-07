use thiserror::Error;

#[derive(Error, Debug)]
pub enum QwobotError {
    #[error("Model loading failed: {0}")]
    ModelLoad(String),

    #[error("Inference failed: {0}")]
    Inference(String),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Model is busy, please try again")]
    ModelBusy,

    #[error("Discord error: {0}")]
    Discord(String),

    #[error("SPW parse failed: {0}")]
    SpwParse(String),
}

pub type Result<T> = std::result::Result<T, QwobotError>;
