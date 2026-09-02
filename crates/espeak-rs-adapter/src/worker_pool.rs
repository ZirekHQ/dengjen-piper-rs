use std::sync::mpsc;
use std::thread;

use piper_core::domain::errors::PhonemizationError;

struct Job {
    text: String,
    voice: String,
    respond_to: mpsc::Sender<Result<Vec<String>, espeak_rs::ESpeakError>>,
}

/// Approach A (design §5): a bounded queue plus one dedicated worker
/// thread. Does not remove the one-caller-at-a-time constraint espeak-ng's
/// process-global state imposes (`crates/espeak-rs`'s own `ESPEAK_LOCK`
/// already serializes correctly) — it makes overload explicit and bounded:
/// a full queue fails fast with `QueueFull` instead of blocking
/// indefinitely on that lock. Never exported from this crate:
/// `EspeakRsPhonemizer` (the crate's only public type) owns one, so
/// swapping this for Approach B's multi-process pool later needs no
/// change to any caller outside this crate.
pub(crate) struct PhonemizerWorkerPool {
    sender: mpsc::SyncSender<Job>,
    _worker: thread::JoinHandle<()>,
}

impl PhonemizerWorkerPool {
    /// `processor` is injected (rather than hardcoded to
    /// `espeak_rs::text_to_phonemes`) so tests can supply a
    /// deterministically-controllable stand-in — a real backend call has no
    /// way to reliably keep the worker "busy" on demand for a `QueueFull`
    /// test.
    pub(crate) fn with_processor(
        capacity: usize,
        processor: impl Fn(&str, &str) -> Result<Vec<String>, espeak_rs::ESpeakError> + Send + 'static,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Job>(capacity);
        let worker = thread::spawn(move || {
            while let Ok(job) = receiver.recv() {
                let result = processor(&job.text, &job.voice);
                let _ = job.respond_to.send(result);
            }
        });
        Self {
            sender,
            _worker: worker,
        }
    }

    // No caller yet within this crate — `EspeakRsPhonemizer` (Task 9) is
    // the one that constructs a production pool via `new`; until then it's
    // unreachable even from the test binary, unlike `with_processor` and
    // `phonemize` below, which the tests do exercise.
    #[allow(dead_code)]
    pub(crate) fn new(capacity: usize) -> Self {
        Self::with_processor(capacity, |text, voice| {
            espeak_rs::text_to_phonemes(text, voice, None)
        })
    }

    pub(crate) fn phonemize(
        &self,
        text: &str,
        voice: &str,
    ) -> Result<Vec<String>, PhonemizationError> {
        let (respond_to, response) = mpsc::channel();
        let job = Job {
            text: text.to_string(),
            voice: voice.to_string(),
            respond_to,
        };
        self.sender
            .try_send(job)
            .map_err(|_| PhonemizationError::QueueFull)?;
        response
            .recv()
            .map_err(|_| {
                PhonemizationError::BackendFailure("worker thread disconnected".to_string())
            })?
            .map_err(|e| PhonemizationError::BackendFailure(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phonemize_returns_the_processors_result_on_success() {
        let pool = PhonemizerWorkerPool::with_processor(4, |text, _voice| {
            Ok(vec![format!("processed: {text}")])
        });

        let result = pool.phonemize("hello", "en-US").unwrap();

        assert_eq!(result, vec!["processed: hello".to_string()]);
    }

    #[test]
    fn phonemize_wraps_a_processor_error_as_backend_failure() {
        let pool = PhonemizerWorkerPool::with_processor(4, |_text, _voice| {
            Err(espeak_rs::ESpeakError("boom".to_string()))
        });

        let result = pool.phonemize("hello", "en-US");

        assert!(
            matches!(result, Err(PhonemizationError::BackendFailure(msg)) if msg.contains("boom"))
        );
    }

    #[test]
    fn a_full_queue_returns_queue_full_without_blocking() {
        // `phonemize` is itself fully synchronous — it blocks its caller
        // on `response.recv()` until the worker replies — so this needs
        // genuinely concurrent callers, not sequential calls from one
        // thread: a single dequeue by the worker instantly frees the
        // channel's buffer slot again, so a *sequential* second call would
        // itself queue successfully and then block on its own response,
        // never reaching a third call at all. Capacity 1: thread A is
        // dequeued and its processor blocks on `release` (test-controlled,
        // not a timing guess); thread B's try_send then fills the queue's
        // one open slot; this test's own thread's try_send for C hits a
        // genuinely full queue and returns immediately (try_send never
        // blocks, so no separate thread is needed for C).
        //
        // The processor closure passed to `with_processor` must be `Send +
        // 'static` (the worker thread it runs on outlives this function's
        // stack frame), so `release_rx` is wrapped for that closure alone —
        // unrelated to the `thread::scope` below, which needs no such
        // wrapping since scoped threads may borrow this function's locals
        // directly.
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let release_rx = std::sync::Mutex::new(release_rx);
        let pool = PhonemizerWorkerPool::with_processor(1, move |_text, _voice| {
            release_rx.lock().unwrap().recv().ok();
            Ok(vec![])
        });

        // Everything up to and including the two `release_tx.send`s has to
        // run *inside* this closure: `thread::scope` joins every thread it
        // spawned only when the closure returns, so code placed after
        // `thread::scope(...)` instead would deadlock — it would never run
        // until A and B complete, but they can't complete until the
        // releases below, which would be sitting in that unreachable code.
        std::thread::scope(|scope| {
            let handle_a = scope.spawn(|| pool.phonemize("a", "en-US"));
            // The one place a short sleep is acceptable: it only widens the
            // window in which the test could flakily pass by coincidence,
            // never the window in which it could flakily fail — job A not
            // yet dequeued would itself still occupy the queue's one slot,
            // so B's try_send failing early would just mean this test's
            // own setup is wrong, not that the pool misbehaved.
            std::thread::sleep(std::time::Duration::from_millis(50));

            let handle_b = scope.spawn(|| pool.phonemize("b", "en-US"));
            std::thread::sleep(std::time::Duration::from_millis(50));

            let result_c = pool.phonemize("c", "en-US");
            assert!(
                matches!(result_c, Err(PhonemizationError::QueueFull)),
                "job C should have hit a full queue: {result_c:?}"
            );

            release_tx.send(()).ok(); // unblocks A's processing
            release_tx.send(()).ok(); // unblocks B's processing once the worker reaches it

            assert!(
                handle_a.join().unwrap().is_ok(),
                "job A should have succeeded"
            );
            assert!(
                handle_b.join().unwrap().is_ok(),
                "job B should have succeeded"
            );
        });
    }
}
