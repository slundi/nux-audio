//! Async response routing: [`MidiDispatcher`] bridges the synchronous midir input
//! callback to tokio consumers through a `broadcast` channel.

use crate::error::MidiError;
use std::time::Duration;
use tokio::sync::broadcast;

/// Buffer size for the broadcast channel that fans out incoming MIDI messages.
///
/// Sized to absorb a full bank dump (128 presets × several SysEx packets each)
/// with headroom in case a subscriber is briefly slow. Previously 64, which
/// could silently overflow during multi-preset fetches.
pub(super) const DISPATCHER_CAPACITY: usize = 256;

/// Outcome of processing one `broadcast::Receiver::recv()` result inside
/// [`wait_for`]. Extracted so the pattern-match branches —
/// especially the silent-drop-on-lag path — can be unit-tested without needing
/// to race a real broadcast channel.
enum RecvStep {
    /// Terminate the loop and return this result to the caller.
    Return(Result<Vec<u8>, MidiError>),
    /// Discard this message and keep waiting.
    Continue,
}

/// Classify a `recv()` result against `pattern`, emitting a stderr warning
/// when tokio reports lag (broadcast overflow → silent message drops).
fn classify_recv(result: Result<Vec<u8>, broadcast::error::RecvError>, pattern: &[u8]) -> RecvStep {
    match result {
        Ok(msg) if msg.starts_with(pattern) => RecvStep::Return(Ok(msg)),
        Ok(_) => RecvStep::Continue,
        Err(broadcast::error::RecvError::Lagged(n)) => {
            // The receiver fell behind by `n` messages; those are lost.
            // Warn the user because a lag during a bank dump means silently
            // missing preset data downstream.
            eprintln!(
                "warning: MIDI dispatcher lagged, dropped {n} message(s) — \
                 consider increasing DISPATCHER_CAPACITY or slowing bulk queries"
            );
            RecvStep::Continue
        }
        Err(broadcast::error::RecvError::Closed) => RecvStep::Return(Err(MidiError::ChannelClosed)),
    }
}

/// Wait on an already-subscribed `rx` for the first message starting with `pattern`.
///
/// Taking the receiver (rather than subscribing here) lets callers subscribe
/// *before* sending a request, so a fast response cannot slip past.
pub(super) async fn wait_for(
    mut rx: broadcast::Receiver<Vec<u8>>,
    pattern: &[u8],
    timeout: Duration,
) -> Result<Vec<u8>, MidiError> {
    let result = tokio::time::timeout(timeout, async move {
        loop {
            match classify_recv(rx.recv().await, pattern) {
                RecvStep::Return(r) => return r,
                RecvStep::Continue => continue,
            }
        }
    })
    .await;
    match result {
        Ok(inner) => inner,
        Err(_elapsed) => Err(MidiError::Timeout(timeout)),
    }
}

/// Async MIDI response dispatcher backed by a `tokio::sync::broadcast` channel.
///
/// Bridges the synchronous midir callback into async tokio consumers.
/// Multiple tasks can subscribe independently; each receives every message.
pub struct MidiDispatcher {
    tx: broadcast::Sender<Vec<u8>>,
}

impl MidiDispatcher {
    pub(super) fn new(tx: broadcast::Sender<Vec<u8>>) -> Self {
        Self { tx }
    }

    /// Subscribe to receive all incoming MIDI messages.
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.tx.subscribe()
    }

    /// Wait asynchronously for the first message whose bytes start with `pattern`.
    ///
    /// Only messages arriving *after* this call are seen; to pair a request with
    /// its response use [`MidiTransport::request_sysex`](super::MidiTransport::request_sysex).
    /// Returns [`MidiError::Timeout`] if no matching message arrives within
    /// `timeout`, and [`MidiError::ChannelClosed`] if the sender is dropped.
    pub async fn await_response(
        &self,
        pattern: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, MidiError> {
        wait_for(self.tx.subscribe(), pattern, timeout).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Matching message → terminate with Ok.
    #[test]
    fn classify_recv_matching_message_returns_ok() {
        let step = classify_recv(Ok(vec![0xF0, 0x43, 0x58, 0x99]), &[0xF0, 0x43, 0x58]);
        match step {
            RecvStep::Return(Ok(msg)) => assert_eq!(msg, vec![0xF0, 0x43, 0x58, 0x99]),
            _ => panic!("expected Return(Ok(...)), got other"),
        }
    }

    /// Non-matching message → keep polling.
    #[test]
    fn classify_recv_unrelated_message_continues() {
        let step = classify_recv(Ok(vec![0xB0, 0x22, 0x50]), &[0xF0, 0x43, 0x58]);
        matches!(step, RecvStep::Continue).then_some(()).unwrap();
    }

    /// Lag → must continue (regression: previously silent, now also warns).
    /// The behavioural invariant is that lag is NOT a hard failure.
    #[test]
    fn classify_recv_lagged_continues_not_returns() {
        let step = classify_recv(
            Err(broadcast::error::RecvError::Lagged(7)),
            &[0xF0, 0x43, 0x58],
        );
        assert!(
            matches!(step, RecvStep::Continue),
            "Lagged must not cause await_response to return — it drops n messages and keeps polling"
        );
    }

    /// Channel closed → surface as `ChannelClosed` so the caller knows the
    /// dispatcher is dead (as opposed to just idle).
    #[test]
    fn classify_recv_closed_returns_channel_closed() {
        let step = classify_recv(
            Err(broadcast::error::RecvError::Closed),
            &[0xF0, 0x43, 0x58],
        );
        assert!(matches!(
            step,
            RecvStep::Return(Err(MidiError::ChannelClosed))
        ));
    }

    /// A message sent between subscribe and wait is still delivered — the
    /// guarantee `request_sysex` relies on.
    #[tokio::test]
    async fn wait_for_sees_message_sent_after_subscribe() {
        let (tx, _) = broadcast::channel::<Vec<u8>>(DISPATCHER_CAPACITY);
        let dispatcher = MidiDispatcher::new(tx.clone());
        let rx = dispatcher.subscribe();
        tx.send(vec![0xB0, 0x01, 0x02]).unwrap();
        tx.send(vec![0xF0, 0x43, 0x58, 0x10, 0xF7]).unwrap();

        let msg = wait_for(rx, &[0xF0, 0x43, 0x58, 0x10], Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(msg, vec![0xF0, 0x43, 0x58, 0x10, 0xF7]);
    }

    #[tokio::test(start_paused = true)]
    async fn await_response_times_out() {
        let (tx, _) = broadcast::channel::<Vec<u8>>(DISPATCHER_CAPACITY);
        let dispatcher = MidiDispatcher::new(tx);
        let result = dispatcher
            .await_response(&[0xF0], Duration::from_millis(50))
            .await;
        assert!(matches!(result, Err(MidiError::Timeout(_))));
    }

    #[tokio::test]
    async fn wait_for_reports_closed_channel() {
        let (tx, rx) = broadcast::channel::<Vec<u8>>(DISPATCHER_CAPACITY);
        drop(tx);
        let result = wait_for(rx, &[0xF0], Duration::from_secs(1)).await;
        assert!(matches!(result, Err(MidiError::ChannelClosed)));
    }
}
