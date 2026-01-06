use super::{log_command_source, Context, Error};
use crate::llm::ModelLoadProgress;
use tracing::{info, warn};

/// Switch to a different LLM model at runtime
#[poise::command(slash_command, prefix_command, rename = "model-switch")]
pub async fn model_switch(
    ctx: Context<'_>,
    #[description = "Model ID (HuggingFace repo like 'Qwen/Qwen3-0.6B' or local path)"]
    model_id: String,
    #[description = "Quantization level (e.g., Q4_K_M, Q8_0)"]
    quantization: String,
) -> Result<(), Error> {
    log_command_source(&ctx, "model-switch");

    let current_model = ctx.data().llm.current_model_id();
    let current_quant = ctx.data().llm.current_quantization();

    info!(
        from_model = %current_model,
        from_quant = %current_quant,
        to_model = %model_id,
        to_quant = %quantization,
        "Model switch requested"
    );

    // Send initial feedback
    let handle = ctx
        .send(
            poise::CreateReply::default().content(format!(
                "Switching model from `{} ({})` to `{} ({})`...\n\nResolving model...",
                current_model, current_quant, model_id, quantization
            )),
        )
        .await?;

    // Create progress channel
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(16);

    // Spawn the model switch task
    let llm = ctx.data().llm.clone();
    let model_id_clone = model_id.clone();
    let quantization_clone = quantization.clone();

    let switch_handle = tokio::spawn(async move {
        llm.switch_model(&model_id_clone, &quantization_clone, progress_tx)
            .await
    });

    // Monitor progress and update the message
    let mut last_status = String::new();
    loop {
        tokio::select! {
            progress = progress_rx.recv() => {
                match progress {
                    Some(ModelLoadProgress::Resolving { model_id }) => {
                        last_status = format!("Resolving `{}`...", model_id);
                        let _ = handle
                            .edit(ctx, poise::CreateReply::default().content(format!(
                                "Switching model from `{} ({})` to `{} ({})`...\n\n{}",
                                current_model, current_quant, model_id, quantization, last_status
                            )))
                            .await;
                    }
                    Some(ModelLoadProgress::Downloading { model_id, filename }) => {
                        last_status = format!("Downloading `{}/{}`...", model_id, filename);
                        let _ = handle
                            .edit(ctx, poise::CreateReply::default().content(format!(
                                "Switching model from `{} ({})` to `{} ({})`...\n\n{}",
                                current_model, current_quant, model_id, quantization, last_status
                            )))
                            .await;
                    }
                    Some(ModelLoadProgress::Loading { path }) => {
                        // Shorten path for display
                        let display_path = if path.len() > 50 {
                            format!("...{}", &path[path.len() - 47..])
                        } else {
                            path
                        };
                        last_status = format!("Loading model from `{}`...", display_path);
                        let _ = handle
                            .edit(ctx, poise::CreateReply::default().content(format!(
                                "Switching model from `{} ({})` to `{} ({})`...\n\n{}",
                                current_model, current_quant, model_id, quantization, last_status
                            )))
                            .await;
                    }
                    Some(ModelLoadProgress::Loaded { vocab, params }) => {
                        let params_display = if params > 1_000_000_000 {
                            format!("{:.1}B", params as f64 / 1_000_000_000.0)
                        } else if params > 1_000_000 {
                            format!("{:.1}M", params as f64 / 1_000_000.0)
                        } else {
                            format!("{}", params)
                        };

                        let _ = handle
                            .edit(ctx, poise::CreateReply::default().content(format!(
                                "Model switched successfully!\n\n\
                                **From:** `{} ({})`\n\
                                **To:** `{} ({})`\n\n\
                                **Vocab:** {}\n\
                                **Params:** {}",
                                current_model, current_quant,
                                model_id, quantization,
                                vocab, params_display
                            )))
                            .await;
                        return Ok(());
                    }
                    Some(ModelLoadProgress::Failed { error }) => {
                        warn!(error = %error, "Model switch failed");
                        let _ = handle
                            .edit(ctx, poise::CreateReply::default().content(format!(
                                "Model switch failed! Keeping original model.\n\n\
                                **Current model:** `{} ({})`\n\n\
                                **Error:** {}",
                                current_model, current_quant, error
                            )))
                            .await;
                        return Ok(());
                    }
                    None => {
                        // Channel closed, wait for task result
                        break;
                    }
                }
            }
        }
    }

    // If we got here, the progress channel closed - check the final result
    match switch_handle.await {
        Ok(Ok(())) => {
            // Success was already reported via progress
            info!("Model switch completed");
        }
        Ok(Err(e)) => {
            warn!(error = %e, "Model switch failed");
            let _ = handle
                .edit(
                    ctx,
                    poise::CreateReply::default().content(format!(
                        "Model switch failed! Keeping original model.\n\n\
                        **Current model:** `{} ({})`\n\n\
                        **Error:** {}",
                        current_model, current_quant, e
                    )),
                )
                .await;
        }
        Err(e) => {
            warn!(error = %e, "Model switch task panicked");
            let _ = handle
                .edit(
                    ctx,
                    poise::CreateReply::default().content(format!(
                        "Model switch failed unexpectedly! Keeping original model.\n\n\
                        **Current model:** `{} ({})`\n\n\
                        **Error:** Task panicked: {}",
                        current_model, current_quant, e
                    )),
                )
                .await;
        }
    }

    Ok(())
}
