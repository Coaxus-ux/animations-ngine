use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Condvar, Mutex, PoisonError, mpsc};
use std::thread;

use anyhow::Result;

/// Frames per work item. A constant, not derived from the thread count, so whatever
/// depends on chunk boundaries (a worker re-rendering the frame before its chunk) never
/// makes the output depend on `--jobs`.
pub(crate) const CHUNK: u32 = 16;

/// Chunks finished ahead of the writer, per thread, before workers wait.
const LOOKAHEAD: u32 = 2;

pub(crate) fn default_jobs() -> NonZeroUsize {
    thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
}

/// Runs `produce` over `0..frames` in `CHUNK`-sized ranges on `jobs` threads, each with its
/// own `make_worker()` state, and hands every item to `consume` in frame order.
pub(crate) fn ordered<W, T>(
    frames: u32,
    jobs: NonZeroUsize,
    make_worker: impl Fn() -> W + Sync,
    produce: impl Fn(&mut W, Range<u32>) -> Result<Vec<T>> + Sync,
    mut consume: impl FnMut(T) -> Result<()>,
) -> Result<()>
where
    T: Send,
{
    let chunks = frames.div_ceil(CHUNK);
    let window = u32::try_from(jobs.get())
        .unwrap_or(u32::MAX)
        .saturating_mul(LOOKAHEAD);
    let next = AtomicU32::new(0);
    let gate = Gate::default();
    thread::scope(|scope| {
        let (sender, receiver) = mpsc::channel();
        for _ in 0..jobs
            .get()
            .min(usize::try_from(chunks).unwrap_or(usize::MAX))
        {
            let sender = sender.clone();
            let (next, gate, make_worker, produce) = (&next, &gate, &make_worker, &produce);
            scope.spawn(move || {
                let mut worker = make_worker();
                loop {
                    let chunk = next.fetch_add(1, Ordering::Relaxed);
                    if chunk >= chunks || !gate.wait_for(chunk, window) {
                        return;
                    }
                    let range = chunk * CHUNK..((chunk + 1) * CHUNK).min(frames);
                    let items = produce(&mut worker, range);
                    if sender.send((chunk, items)).is_err() {
                        return;
                    }
                }
            });
        }
        drop(sender);
        let result = drain(&receiver, chunks, &gate, &mut consume);
        gate.close();
        result
    })
}

fn drain<T>(
    receiver: &mpsc::Receiver<(u32, Result<Vec<T>>)>,
    chunks: u32,
    gate: &Gate,
    consume: &mut impl FnMut(T) -> Result<()>,
) -> Result<()> {
    let mut pending = BTreeMap::new();
    let mut expected = 0;
    while expected < chunks {
        let Ok((chunk, items)) = receiver.recv() else {
            break;
        };
        pending.insert(chunk, items);
        while let Some(items) = pending.remove(&expected) {
            for item in items? {
                consume(item)?;
            }
            expected += 1;
            gate.advance(expected);
        }
    }
    Ok(())
}

/// How far the writer got; workers wait on it so finished chunks can't pile up unbounded.
#[derive(Default)]
struct Gate {
    state: Mutex<GateState>,
    changed: Condvar,
}

#[derive(Default)]
struct GateState {
    consumed: u32,
    closed: bool,
}

impl Gate {
    /// Waits until `chunk` is within `window` of the writer; false once the run is over.
    fn wait_for(&self, chunk: u32, window: u32) -> bool {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let state = self
            .changed
            .wait_while(state, |state| {
                !state.closed && chunk >= state.consumed.saturating_add(window)
            })
            .unwrap_or_else(PoisonError::into_inner);
        !state.closed
    }

    fn advance(&self, consumed: u32) {
        self.update(|state| state.consumed = consumed);
    }

    fn close(&self) {
        self.update(|state| state.closed = true);
    }

    fn update(&self, change: impl FnOnce(&mut GateState)) {
        change(&mut self.state.lock().unwrap_or_else(PoisonError::into_inner));
        self.changed.notify_all();
    }
}
