use std::sync::mpsc;
use std::thread;

use piper_core::domain::errors::PhonemizationError;

struct Job {
    text: String,
    voice: String,
    respond_to: mpsc::Sender<Result<Vec<String>, espeak_rs::ESpeakError>>,
}

pub(crate) struct PhonemizerWorkerPool {
    sender: mpsc::SyncSender<Job>,
    _worker: thread::JoinHandle<()>,
}

impl PhonemizerWorkerPool {
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
        self.sender.try_send(job).map_err(|e| match e {
            mpsc::TrySendError::Full(_) => PhonemizationError::QueueFull,
            mpsc::TrySendError::Disconnected(_) => {
                PhonemizationError::BackendFailure("worker thread disconnected".to_string())
            }
        })?;
        response
            .recv()
            .map_err(|_| {
                PhonemizationError::BackendFailure("worker thread disconnected".to_string())
            })?
            .map_err(|e| {
                if e.is_timeout() {
                    PhonemizationError::Timeout
                } else {
                    PhonemizationError::BackendFailure(e.to_string())
                }
            })
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
    fn phonemize_wraps_a_processor_failure_as_backend_failure() {
        let pool = PhonemizerWorkerPool::with_processor(4, |_text, _voice| {
            Err(espeak_rs::ESpeakError::Failure("boom".to_string()))
        });

        let result = pool.phonemize("hello", "en-US");

        assert!(
            matches!(result, Err(PhonemizationError::BackendFailure(msg)) if msg.contains("boom"))
        );
    }

    /// Closes #52: a backend timeout must reach the caller as
    /// `PhonemizationError::Timeout`, not fold into the same
    /// `BackendFailure` bucket as every other backend error, so callers can
    /// tell "never responded" apart from other failures (e.g. to map it onto
    /// a distinct HTTP status downstream).
    #[test]
    fn phonemize_maps_a_processor_timeout_to_phonemization_timeout() {
        let pool = PhonemizerWorkerPool::with_processor(4, |_text, _voice| {
            Err(espeak_rs::ESpeakError::Timeout("timed out".to_string()))
        });

        let result = pool.phonemize("hello", "en-US");

        assert!(matches!(result, Err(PhonemizationError::Timeout)));
    }

    #[test]
    fn a_full_queue_returns_queue_full_without_blocking() {
        // `phonemize` is itself fully synchronous — it blocks its caller
        // on `response.recv()` until the worker replies — so this needs
        // genuinely concurrent callers, not sequential calls from one
        // thread: a single dequeue by the worker instantly frees the
        // channel's buffer slot again, so a *sequential* second call would
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let release_rx = std::sync::Mutex::new(release_rx);
        let pool = PhonemizerWorkerPool::with_processor(1, move |_text, _voice| {
            release_rx.lock().unwrap().recv().ok();
            Ok(vec![])
        });

        std::thread::scope(|scope| {
            let handle_a = scope.spawn(|| pool.phonemize("a", "en-US"));
            std::thread::sleep(std::time::Duration::from_millis(50));

            let handle_b = scope.spawn(|| pool.phonemize("b", "en-US"));
            std::thread::sleep(std::time::Duration::from_millis(50));

            let result_c = pool.phonemize("c", "en-US");
            assert!(
                matches!(result_c, Err(PhonemizationError::QueueFull)),
                "job C should have hit a full queue: {result_c:?}"
            );

            release_tx.send(()).ok(); 
            release_tx.send(()).ok(); 

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
