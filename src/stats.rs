//! Command statistics and failure tracking.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Maximum number of recent failures to keep.
const MAX_RECENT_FAILURES: usize = 50;

/// Tracks command execution statistics.
#[derive(Debug, Clone)]
pub struct CommandStats {
    inner: Arc<RwLock<StatsInner>>,
    start_time: Instant,
}

#[derive(Debug, Default)]
struct StatsInner {
    /// Per-command statistics.
    commands: std::collections::HashMap<String, CommandStat>,
    /// Recent failures with context.
    recent_failures: VecDeque<FailureRecord>,
}

#[derive(Debug, Default, Clone)]
struct CommandStat {
    /// Total invocations.
    invocations: u64,
    /// Successful completions.
    successes: u64,
    /// Failed executions.
    failures: u64,
    /// Total response time in milliseconds.
    total_latency_ms: u64,
    /// Minimum response time.
    min_latency_ms: Option<u64>,
    /// Maximum response time.
    max_latency_ms: Option<u64>,
}

/// Record of a command failure.
#[derive(Debug, Clone)]
pub struct FailureRecord {
    /// Command name.
    pub command: String,
    /// Error message.
    pub error: String,
    /// User who triggered it (ID as string).
    pub user_id: String,
    /// When it happened.
    pub timestamp: std::time::SystemTime,
    /// How long the command ran before failing.
    pub duration_ms: u64,
}

impl CommandStats {
    /// Creates a new stats tracker.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(StatsInner::default())),
            start_time: Instant::now(),
        }
    }

    /// Records the start of a command execution.
    pub async fn record_invocation(&self, command: &str) {
        let mut inner = self.inner.write().await;
        let stat = inner.commands.entry(command.to_string()).or_default();
        stat.invocations += 1;
    }

    /// Records a successful command completion.
    pub async fn record_success(&self, command: &str, duration: Duration) {
        let mut inner = self.inner.write().await;
        let stat = inner.commands.entry(command.to_string()).or_default();
        stat.successes += 1;

        let ms = duration.as_millis() as u64;
        stat.total_latency_ms += ms;
        stat.min_latency_ms = Some(stat.min_latency_ms.map_or(ms, |m| m.min(ms)));
        stat.max_latency_ms = Some(stat.max_latency_ms.map_or(ms, |m| m.max(ms)));
    }

    /// Records a command failure.
    pub async fn record_failure(
        &self,
        command: &str,
        error: &str,
        user_id: &str,
        duration: Duration,
    ) {
        let mut inner = self.inner.write().await;
        let stat = inner.commands.entry(command.to_string()).or_default();
        stat.failures += 1;

        let ms = duration.as_millis() as u64;
        stat.total_latency_ms += ms;
        stat.min_latency_ms = Some(stat.min_latency_ms.map_or(ms, |m| m.min(ms)));
        stat.max_latency_ms = Some(stat.max_latency_ms.map_or(ms, |m| m.max(ms)));

        // Add to recent failures
        let record = FailureRecord {
            command: command.to_string(),
            error: error.to_string(),
            user_id: user_id.to_string(),
            timestamp: std::time::SystemTime::now(),
            duration_ms: ms,
        };

        inner.recent_failures.push_back(record);
        while inner.recent_failures.len() > MAX_RECENT_FAILURES {
            inner.recent_failures.pop_front();
        }
    }

    /// Returns the bot's uptime.
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Gets a summary of all stats.
    pub async fn summary(&self) -> StatsSummary {
        let inner = self.inner.read().await;

        let mut total_invocations = 0u64;
        let mut total_successes = 0u64;
        let mut total_failures = 0u64;
        let mut command_stats = Vec::new();

        for (name, stat) in &inner.commands {
            total_invocations += stat.invocations;
            total_successes += stat.successes;
            total_failures += stat.failures;

            let avg_latency = if stat.invocations > 0 {
                stat.total_latency_ms / stat.invocations
            } else {
                0
            };

            command_stats.push(CommandStatSummary {
                name: name.clone(),
                invocations: stat.invocations,
                successes: stat.successes,
                failures: stat.failures,
                avg_latency_ms: avg_latency,
                min_latency_ms: stat.min_latency_ms.unwrap_or(0),
                max_latency_ms: stat.max_latency_ms.unwrap_or(0),
            });
        }

        // Sort by invocations descending
        command_stats.sort_by(|a, b| b.invocations.cmp(&a.invocations));

        StatsSummary {
            uptime: self.uptime(),
            total_invocations,
            total_successes,
            total_failures,
            commands: command_stats,
            recent_failures: inner.recent_failures.iter().rev().take(10).cloned().collect(),
        }
    }
}

impl Default for CommandStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of command statistics.
#[derive(Debug)]
pub struct StatsSummary {
    pub uptime: Duration,
    pub total_invocations: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub commands: Vec<CommandStatSummary>,
    pub recent_failures: Vec<FailureRecord>,
}

impl StatsSummary {
    /// Returns the overall success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        if self.total_invocations == 0 {
            100.0
        } else {
            (self.total_successes as f64 / self.total_invocations as f64) * 100.0
        }
    }

    /// Formats the uptime as a human-readable string.
    pub fn uptime_display(&self) -> String {
        let secs = self.uptime.as_secs();
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let mins = (secs % 3600) / 60;
        let secs = secs % 60;

        if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, mins, secs)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, mins, secs)
        } else if mins > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}s", secs)
        }
    }
}

/// Per-command statistics summary.
#[derive(Debug)]
pub struct CommandStatSummary {
    pub name: String,
    pub invocations: u64,
    pub successes: u64,
    pub failures: u64,
    pub avg_latency_ms: u64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
}

impl CommandStatSummary {
    /// Returns the success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        if self.invocations == 0 {
            100.0
        } else {
            (self.successes as f64 / self.invocations as f64) * 100.0
        }
    }
}
