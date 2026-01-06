# qwobot

A Discord bot serving local LLMs via llama.cpp, with special support for the SPW symbolic expression language.

---

## For Users

### Commands

| Command | Description |
|---------|-------------|
| `/complete <prompt>` | Generate text from a prompt using the loaded LLM |
| `/spweeboard <expression> [ground]` | Interpret an SPW symbolic expression |
| `/model` | Display information about the currently loaded model |
| `/model-switch <model_id> <quantization>` | Switch to a different model at runtime |
| `/prompt-refresh` | Manually refresh the SPW prompt template from remote |
| `/bot-status` | View bot health, command statistics, and recent failures |

### Using `/complete`

Generate text completions from any prompt:

```
/complete Tell me a joke about programming
```

The bot will show a preview of your prompt and the model's response.

### Using `/spweeboard`

SPW (Symbolic Poetic Writing) interprets symbolic expressions as natural language. Symbols encode cognitive operations:

| Symbol | Meaning |
|--------|---------|
| `~` | potential/becoming |
| `#` | vibration/resonance |
| `.` | ground/foundation |
| `?` | wonder/inquiry |
| `!` | action/assertion |
| `*` | value/significance |
| `&` | subject/agent |
| `@` | perspective/view |
| `<>` | concept/abstraction |
| `()` | scene/situation |
| `[]` | mode/manner |
| `{}` | direction/intent |
| `^` | integration/synthesis |

**Examples:**

```
/spweeboard &@
→ "From where I stand, looking out at what unfolds before me."

/spweeboard ?!
→ "A pause of wonder, then decisive movement forward."

/spweeboard expression:&*[work] ground:.{home}
→ Interprets the expression with "home" as the grounding context
```

Order matters: `&@` = "from subject's view", `@&` = "the subject being viewed".

### Checking Bot Status

Use `/bot-status` to see:
- Bot uptime and current model
- Command success/failure rates
- Recent errors (helpful for troubleshooting)

---

## For Deployers

### Requirements

- Rust 1.75+ (for building)
- A Discord bot token
- GPU recommended (Metal on macOS, CUDA on Linux/Windows)

### Quick Start

1. **Clone and build:**
   ```bash
   git clone <repo-url>
   cd qwobot
   cargo build --release
   ```

2. **Configure environment:**
   ```bash
   cp .env.example .env
   # Edit .env and add your Discord token:
   # DISCORD_TOKEN=your_token_here
   ```

3. **Configure the bot** (optional - defaults work out of the box):
   ```bash
   # Edit config/default.toml to customize model, temperature, etc.
   ```

4. **Run:**
   ```bash
   cargo run --release
   ```

### Configuration

Configuration is loaded from `config/default.toml` with environment variable overrides using the `QWOBOT__` prefix.

#### config/default.toml

```toml
# Model settings
# model_id can be:
#   - Local GGUF file path: "/path/to/model.gguf"
#   - HuggingFace repo: "Qwen/Qwen3-0.6B" (will look for GGUF in {repo}-GGUF)
model_id = "Qwen/Qwen3-0.6B"

# Quantization level (used to find the right GGUF file)
# Common values: Q4_K_M, Q8_0, Q6_K, Q5_K_M, Q3_K_M, Q2_K
quantization = "Q8_0"

# Generation settings
max_tokens = 1024
temperature = 0.7

# SPW prompt template URL (checked periodically for updates)
prompt_url = "https://raw.githubusercontent.com/spwplace/spweeboard/refs/heads/dev/crates/spweeboard-core/src/compiler/prompt.txt"

# How often to check for prompt updates (seconds)
prompt_poll_interval_secs = 60

# Optional prefix for text commands (e.g., "!" enables !complete)
# prefix = "!"
```

#### Environment Variables

| Variable | Description |
|----------|-------------|
| `DISCORD_TOKEN` | **Required.** Your Discord bot token |
| `QWOBOT__MODEL_ID` | Override model_id |
| `QWOBOT__QUANTIZATION` | Override quantization |
| `QWOBOT__MAX_TOKENS` | Override max_tokens |
| `QWOBOT__TEMPERATURE` | Override temperature |
| `QWOBOT__PREFIX` | Override command prefix |
| `RUST_LOG` | Logging level (e.g., `info`, `debug`, `qwobot=trace`) |

### Model Selection

The bot automatically downloads models from HuggingFace. For a model like `Qwen/Qwen3-0.6B`:

1. It first tries the GGUF variant repo: `Qwen/Qwen3-0.6B-GGUF`
2. Looks for files matching the quantization (e.g., `Qwen3-0.6B-Q8_0.gguf`)
3. Falls back to the original repo if needed

**Recommended models:**
- `Qwen/Qwen3-0.6B` (Q8_0) - Small, fast, good for testing
- `Qwen/Qwen3-1.7B` (Q4_K_M) - Better quality, still fast
- `mistralai/Mistral-7B-v0.1` (Q4_K_M) - High quality, needs more VRAM

### Runtime Model Switching

Deployers (or trusted users) can switch models without restarting:

```
/model-switch Qwen/Qwen3-1.7B Q4_K_M
```

The bot will:
1. Download the new model (if needed)
2. Load it into memory
3. Swap to it atomically
4. Roll back automatically if loading fails

### Monitoring

- Use `/bot-status` to monitor command success rates and recent failures
- Check logs for detailed error information
- The bot logs at `info` level by default; use `RUST_LOG=qwobot=debug` for more detail

---

## For Developers

### Project Structure

```
qwobot/
├── src/
│   ├── main.rs           # Entry point, Discord framework setup
│   ├── config.rs         # Configuration loading
│   ├── llm.rs            # LLM engine (llama.cpp wrapper)
│   ├── error.rs          # Error types
│   ├── stats.rs          # Command statistics tracking
│   ├── prompt_watcher.rs # Remote prompt polling
│   └── commands/
│       ├── mod.rs        # Command exports, shared Data struct
│       ├── generate.rs   # /complete command
│       ├── model.rs      # /model command
│       ├── model_switch.rs    # /model-switch command
│       ├── spweeboard.rs      # /spweeboard command
│       ├── prompt_refresh.rs  # /prompt-refresh command
│       └── bot_status.rs      # /bot-status command
├── config/
│   └── default.toml      # Default configuration
├── Cargo.toml
└── README.md
```

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `poise` | Discord bot framework (built on serenity) |
| `llama-cpp-2` | Rust bindings for llama.cpp |
| `hf-hub` | HuggingFace model downloading |
| `spweeboard-core` | SPW expression parsing and prompt compilation |
| `tokio` | Async runtime |
| `reqwest` | HTTP client for prompt fetching |
| `tracing` | Structured logging |

### Architecture

#### LLM Engine (`llm.rs`)

The `LlmEngine` wraps llama.cpp with:
- Thread-safe model access via `Arc<Mutex<LlmInner>>`
- Automatic model downloading from HuggingFace
- GPU offloading (all layers by default)
- Hot-swappable models via `switch_model()`

```rust
// Creating the engine
let engine = LlmEngine::new(model_id, quantization, max_tokens, temperature).await?;

// Generating text
let output = engine.generate(prompt).await?;

// Switching models at runtime
engine.switch_model(new_model_id, new_quantization, progress_tx).await?;
```

#### Command Stats (`stats.rs`)

Tracks per-command metrics:
- Invocation counts
- Success/failure rates
- Response latency
- Recent failures with context

Stats are automatically recorded via pre/post command hooks in `main.rs`.

#### Prompt Watcher (`prompt_watcher.rs`)

Background task that:
1. Fetches the SPW prompt template from a remote URL
2. Compares SHA-256 hash to detect changes
3. Updates shared `PromptState` when changes detected

The `/spweeboard` command reads the current template from `PromptState` on each invocation.

### Adding a New Command

1. Create `src/commands/your_command.rs`:
   ```rust
   use super::{log_command_source, Context, Error};

   #[poise::command(slash_command, prefix_command)]
   pub async fn your_command(
       ctx: Context<'_>,
       #[description = "Parameter description"]
       param: String,
   ) -> Result<(), Error> {
       log_command_source(&ctx, "your_command");
       ctx.defer().await?; // For long operations

       // Your logic here
       ctx.say("Response").await?;
       Ok(())
   }
   ```

2. Export in `src/commands/mod.rs`:
   ```rust
   mod your_command;
   pub use your_command::your_command;
   ```

3. Register in `main.rs`:
   ```rust
   commands: vec![
       // ... existing commands
       commands::your_command(),
   ],
   ```

### Testing

```bash
# Run tests
cargo test

# Run with verbose logging
RUST_LOG=qwobot=trace cargo run

# Check for issues
cargo clippy
```

### The spweeboard-core Dependency

This bot depends on `spweeboard-core` from `../spweebo'ard/crates/spweeboard-core`. Key types:

- `SpweeboardEngine<I>` - Main engine coordinating parsing and inference
- `PromptCompiler` - Compiles SPW expressions into LLM prompts
- `Expression` - Parsed SPW expression
- `Ground` - Context for interpretation

The `PromptCompiler` accepts runtime templates via:
```rust
let compiler = PromptCompiler::with_template(template_string);
let engine = SpweeboardEngine::with_compiler(inference, compiler);
```

---

## License

MIT
