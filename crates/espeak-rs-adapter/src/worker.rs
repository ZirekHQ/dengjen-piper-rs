use std::sync::mpsc;
use std::thread;

use piper_core::domain::errors::PhonemizationError;

struct Job {
    text: String,
    voice: String,
    respond_to: mpsc::Sender<Result<Vec<String>, espeak_rs::ESpeakError>>,
}

/// Dispatches every phonemization call to a single background thread.
///
/// Pinned to exactly one worker, not a scalable pool: espeak-ng holds global
/// mutable C state guarded by one process-wide mutex, so a second worker
/// thread would just contend on that lock rather than add throughput.
pub(crate) struct PhonemizerWorker {
    sender: mpsc::SyncSender<Job>,
    _worker: thread::JoinHandle<()>,
}

impl PhonemizerWorker {
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
        let pool = PhonemizerWorker::with_processor(4, |text, _voice| {
            Ok(vec![format!("processed: {text}")])
        });

        let result = pool.phonemize("hello", "en-US").unwrap();

        assert_eq!(result, vec!["processed: hello".to_string()]);
    }

    #[test]
    fn phonemize_wraps_a_processor_failure_as_backend_failure() {
        let pool = PhonemizerWorker::with_processor(4, |_text, _voice| {
            Err(espeak_rs::ESpeakError::Failure("boom".to_string()))
        });

        let result = pool.phonemize("hello", "en-US");

        assert!(
            matches!(result, Err(PhonemizationError::BackendFailure(msg)) if msg.contains("boom"))
        );
    }

    #[test]
    fn phonemize_maps_a_processor_timeout_to_phonemization_timeout() {
        let pool = PhonemizerWorker::with_processor(4, |_text, _voice| {
            Err(espeak_rs::ESpeakError::Timeout("timed out".to_string()))
        });

        let result = pool.phonemize("hello", "en-US");

        assert!(matches!(result, Err(PhonemizationError::Timeout)));
    }

    #[test]
    fn a_full_queue_returns_queue_full_without_blocking() {
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let release_rx = std::sync::Mutex::new(release_rx);
        let (processing_tx, processing_rx) = mpsc::channel::<()>();
        let pool = PhonemizerWorker::with_processor(1, move |_text, _voice| {
            processing_tx.send(()).ok();
            release_rx.lock().unwrap().recv().ok();
            Ok(vec![])
        });

        // Job A is picked up by the worker immediately, freeing the one
        // buffer slot; wait for it to signal that it is being processed
        // before relying on that slot being empty.
        let (a_respond_to, a_response) = mpsc::channel();
        pool.sender
            .try_send(Job {
                text: "a".to_string(),
                voice: "en-US".to_string(),
                respond_to: a_respond_to,
            })
            .expect("job A should enqueue into the empty buffer");
        processing_rx
            .recv()
            .expect("job A should start processing before job B is enqueued");

        // Job B now fills the single buffer slot while A is still processing.
        let (b_respond_to, b_response) = mpsc::channel();
        pool.sender
            .try_send(Job {
                text: "b".to_string(),
                voice: "en-US".to_string(),
                respond_to: b_respond_to,
            })
            .expect("job B should enqueue into the buffer vacated by job A");

        let result_c = pool.phonemize("c", "en-US");
        assert!(
            matches!(result_c, Err(PhonemizationError::QueueFull)),
            "job C should have hit a full queue: {result_c:?}"
        );

        release_tx.send(()).ok();
        release_tx.send(()).ok();

        assert!(
            a_response.recv().unwrap().is_ok(),
            "job A should have succeeded"
        );
        assert!(
            b_response.recv().unwrap().is_ok(),
            "job B should have succeeded"
        );
    }
}
