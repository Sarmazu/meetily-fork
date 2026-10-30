//! Small shared byte-transfer and detokenization primitives; managers own policy.
use futures_util::{Stream, StreamExt};
use std::{future::Future, time::Duration};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
#[error("Download cancelled by user")]
pub(crate) struct DownloadCancelled;

pub(crate) async fn copy_stream<S, B, E, W, F, Fut>(
    mut stream: S, writer: &mut W, initial: u64, exact: u64,
    cancel: &CancellationToken, idle_timeout: Option<Duration>, mut on_bytes: F,
) -> anyhow::Result<u64>
where
    S: Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>, E: std::fmt::Display,
    W: AsyncWrite + Unpin,
    F: FnMut(u64, u64) -> Fut, Fut: Future<Output = anyhow::Result<()>>,
{
    let result = async {
        let mut size = initial;
        if size > exact { anyhow::bail!("Existing size exceeds exact byte size"); }
        loop {
            let next = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(DownloadCancelled.into()),
                chunk = async {
                    match idle_timeout {
                        Some(duration) => tokio::time::timeout(duration, stream.next()).await
                            .map_err(|_| anyhow::anyhow!("Download timeout: no data received")),
                        None => Ok(stream.next().await),
                    }
                } => chunk?,
            };
            let Some(chunk) = next else { break };
            let chunk = chunk.map_err(|e| anyhow::anyhow!("Download stream failed: {e}"))?;
            let bytes = chunk.as_ref();
            size = size.checked_add(bytes.len() as u64).ok_or_else(|| anyhow::anyhow!("Download size overflow"))?;
            if size > exact { anyhow::bail!("Download response exceeds exact byte size"); }
            writer.write_all(bytes).await?;
            on_bytes(bytes.len() as u64, size).await?;
        }
        if size != exact { anyhow::bail!("Incomplete download: expected {exact} bytes, got {size}"); }
        Ok(size)
    }.await;
    // Preserve resumable bytes on cancellation, timeout and stream errors too.
    writer.flush().await?;
    result
}

pub(crate) fn cleanup_decode_text(text: &str) -> String {
    static SPACE: once_cell::sync::Lazy<Result<regex::Regex, regex::Error>> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"\A\s|\s\B|(\s)\b"));
    match &*SPACE {
        Ok(regex) => regex.replace_all(text, |caps: &regex::Captures<'_>| {
            if caps.get(1).is_some() { " " } else { "" }
        }).into_owned(),
        Err(_) => text.to_owned(),
    }
}
