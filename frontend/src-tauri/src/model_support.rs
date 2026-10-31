//! Small shared byte-transfer and detokenization primitives; managers own policy.
use futures_util::{Stream, StreamExt};
use std::{future::Future, time::Duration};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
#[error("Download cancelled by user")]
pub(crate) struct DownloadCancelled;

pub(crate) async fn copy_stream<S, B, E, W, F, Fut>(
    mut stream: S,
    writer: &mut W,
    initial: u64,
    exact: u64,
    cancel: &CancellationToken,
    idle_timeout: Option<Duration>,
    mut on_bytes: F,
) -> anyhow::Result<u64>
where
    S: Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    E: std::fmt::Display,
    W: AsyncWrite + Unpin,
    F: FnMut(u64, u64) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let result = async {
        let mut size = initial;
        if size > exact {
            anyhow::bail!("Existing size exceeds exact byte size");
        }
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
            size = size
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| anyhow::anyhow!("Download size overflow"))?;
            if size > exact {
                anyhow::bail!("Download response exceeds exact byte size");
            }
            writer.write_all(bytes).await?;
            on_bytes(bytes.len() as u64, size).await?;
        }
        if size != exact {
            anyhow::bail!("Incomplete download: expected {exact} bytes, got {size}");
        }
        Ok(size)
    }
    .await;
    // Preserve resumable bytes on cancellation, timeout and stream errors too.
    writer.flush().await?;
    result
}

pub(crate) fn cleanup_decode_text(text: &str) -> String {
    static SPACE: once_cell::sync::Lazy<Result<regex::Regex, regex::Error>> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"\A\s|\s\B|(\s)\b"));
    match &*SPACE {
        Ok(regex) => regex
            .replace_all(
                text,
                |caps: &regex::Captures<'_>| {
                    if caps.get(1).is_some() {
                        " "
                    } else {
                        ""
                    }
                },
            )
            .into_owned(),
        Err(_) => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    #[derive(Default)]
    struct Writer {
        pending: Vec<u8>,
        flushed: Vec<u8>,
        flushes: usize,
    }
    impl AsyncWrite for Writer {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            self.pending.extend_from_slice(bytes);
            Poll::Ready(Ok(bytes.len()))
        }
        fn poll_flush(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            let pending = std::mem::take(&mut self.pending);
            self.flushed.extend(pending);
            self.flushes += 1;
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            self.poll_flush(cx)
        }
    }
    fn ignore_bytes(_: u64, _: u64) -> std::future::Ready<anyhow::Result<()>> {
        std::future::ready(Ok(()))
    }
    #[tokio::test]
    async fn resumed_transfer_reports_only_new_bytes_and_exact_final_size() {
        let mut writer = Writer {
            flushed: b"ab".to_vec(),
            ..Writer::default()
        };
        let mut progress = Vec::new();
        let result = copy_stream(
            stream::iter([Ok::<_, &str>(b"cd".as_slice()), Ok(b"e".as_slice())]),
            &mut writer,
            2,
            5,
            &CancellationToken::new(),
            None,
            |delta, size| {
                progress.push((delta, size));
                std::future::ready(Ok(()))
            },
        )
        .await
        .unwrap();
        assert_eq!(result, 5);
        assert_eq!(progress, [(2, 4), (1, 5)]);
        assert_eq!(writer.flushed, b"abcde");
        assert_eq!(writer.flushes, 1);
    }
    #[tokio::test]
    async fn short_overlong_and_stream_errors_flush_accepted_bytes() {
        for (chunks, expected_error) in [
            (vec![Ok(b"ab".as_slice())], "Incomplete download"),
            (
                vec![Ok(b"ab".as_slice()), Ok(b"cde".as_slice())],
                "exceeds exact byte size",
            ),
            (
                vec![Ok(b"ab".as_slice()), Err("broken stream")],
                "broken stream",
            ),
        ] {
            let mut writer = Writer::default();
            let mut progress = Vec::new();
            let error = copy_stream(
                stream::iter(chunks),
                &mut writer,
                0,
                4,
                &CancellationToken::new(),
                None,
                |delta, size| {
                    progress.push((delta, size));
                    std::future::ready(Ok(()))
                },
            )
            .await
            .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(progress, [(2, 2)]);
            assert_eq!(writer.flushed, b"ab");
            assert!(writer.pending.is_empty());
            assert_eq!(writer.flushes, 1);
        }
    }
    #[tokio::test]
    async fn cancellation_preserves_bytes_and_typed_error_with_artifact_context() {
        let token = CancellationToken::new();
        let mut writer = Writer::default();
        let error = copy_stream(
            stream::iter([Ok::<_, &str>(b"ab".as_slice()), Ok(b"cd".as_slice())]),
            &mut writer,
            0,
            4,
            &token,
            None,
            |_, _| {
                token.cancel();
                std::future::ready(Ok(()))
            },
        )
        .await
        .unwrap_err()
        .context("Failed to download encoder-model.onnx");
        assert!(error.downcast_ref::<DownloadCancelled>().is_some());
        assert!(error.to_string().contains("encoder-model.onnx"));
        assert_eq!(writer.flushed, b"ab");
        assert_eq!(writer.flushes, 1);
    }
    #[tokio::test]
    async fn idle_timeout_flushes_preceding_bytes() {
        let chunks = stream::iter([Ok::<_, &str>(b"ab".as_slice())]).chain(stream::pending());
        let mut writer = Writer::default();
        let error = copy_stream(
            chunks,
            &mut writer,
            0,
            4,
            &CancellationToken::new(),
            Some(Duration::from_millis(5)),
            ignore_bytes,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("Download timeout"));
        assert_eq!(writer.flushed, b"ab");
        assert_eq!(writer.flushes, 1);
    }
}
