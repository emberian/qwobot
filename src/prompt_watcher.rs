//! Prompt watcher for periodically fetching and updating SPW prompt templates.

use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Shared state for the current prompt template.
#[derive(Debug, Clone)]
pub struct PromptState {
    inner: Arc<RwLock<PromptStateInner>>,
}

#[derive(Debug)]
struct PromptStateInner {
    template: String,
    hash: [u8; 32],
}

impl PromptState {
    /// Creates a new prompt state with the default template.
    pub fn new() -> Self {
        let template = spweeboard_core::PromptCompiler::default().template().to_string();
        let hash = hash_template(&template);
        Self {
            inner: Arc::new(RwLock::new(PromptStateInner { template, hash })),
        }
    }

    /// Gets the current prompt template.
    pub async fn template(&self) -> String {
        self.inner.read().await.template.clone()
    }

    /// Updates the template if the hash has changed. Returns true if updated.
    pub async fn update_if_changed(&self, new_template: String) -> bool {
        let new_hash = hash_template(&new_template);
        let mut inner = self.inner.write().await;

        if inner.hash != new_hash {
            info!(
                old_hash = hex::encode(inner.hash),
                new_hash = hex::encode(new_hash),
                "Prompt template changed, updating"
            );
            inner.template = new_template;
            inner.hash = new_hash;
            true
        } else {
            debug!("Prompt template unchanged");
            false
        }
    }
}

impl Default for PromptState {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_template(template: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(template.as_bytes());
    hasher.finalize().into()
}

/// Fetches the prompt template from a URL.
pub async fn fetch_prompt(url: &str) -> Result<String, reqwest::Error> {
    debug!(url = url, "Fetching prompt template");
    let response = reqwest::get(url).await?;
    let text = response.text().await?;
    debug!(len = text.len(), "Fetched prompt template");
    Ok(text)
}

/// Spawns a background task that periodically checks for prompt updates.
pub fn spawn_watcher(
    url: String,
    interval: Duration,
    state: PromptState,
) -> tokio::task::JoinHandle<()> {
    info!(
        url = url,
        interval_secs = interval.as_secs(),
        "Starting prompt watcher"
    );

    tokio::spawn(async move {
        // Initial fetch
        match fetch_prompt(&url).await {
            Ok(template) => {
                if state.update_if_changed(template).await {
                    info!("Initial prompt template loaded from remote");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch initial prompt, using default");
            }
        }

        // Periodic polling
        let mut interval_timer = tokio::time::interval(interval);
        interval_timer.tick().await; // Skip the first immediate tick

        loop {
            interval_timer.tick().await;

            match fetch_prompt(&url).await {
                Ok(template) => {
                    if state.update_if_changed(template).await {
                        info!("Prompt template updated from remote");
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to fetch prompt template");
                }
            }
        }
    })
}

/// Simple hex encoding for logging hashes.
mod hex {
    pub fn encode(bytes: [u8; 32]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
