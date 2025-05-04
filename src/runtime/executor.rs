//! Multi-threaded executor implementation using async-task.
//! This executor can run futures concurrently across multiple threads.

use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use async_task::Runnable;
use crossbeam_deque::{Injector, Steal, Worker};
use once_cell::sync::Lazy;

use crate::parker::Parker;

// Re-export Task type for public use
pub use async_task::Task;

// Global task queue shared across all worker threads
static GLOBAL_QUEUE: Lazy<Injector<Runnable>> = Lazy::new(|| Injector::new());

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Multi-threaded executor for running async tasks
pub struct Executor {
    num_threads: usize,
    handles: Vec<thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

impl Executor {
    /// Create a new executor with the specified number of worker threads.
    pub fn new() -> Self {
        Self::with_threads(num_cpus::get())
    }

    /// Create a new executor with the specified number of worker threads.
    pub fn with_threads(num_threads: usize) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let handles = Vec::new();

        Self {
            num_threads,
            handles,
            running,
        }
    }

    /// Start worker threads
    pub fn start(&mut self) {
        if !self.handles.is_empty() {
            return;
        }

        let running = self.running.clone();

        for i in 0..self.num_threads {
            let local = Worker::new_fifo();
            let running = running.clone();

            let handle = thread::Builder::new()
                .name(format!("executor-{}", i))
                .spawn(move || {
                    Self::worker_loop(i, local, running);
                })
                .expect("Failed to spawn worker thread");

            self.handles.push(handle);
        }
    }

    /// The main worker loop that steals tasks and executes them
    fn worker_loop(id: usize, local: Worker<Runnable>, running: Arc<AtomicBool>) {
        println!("Worker thread {} started", id);

        while running.load(Ordering::SeqCst) {
            // First check the local queue
            if let Some(runnable) = local.pop() {
                runnable.run();
                continue;
            }

            // Then try to steal from the global queue
            match GLOBAL_QUEUE.steal_batch_and_pop(&local) {
                Steal::Success(runnable) => {
                    runnable.run();
                }
                Steal::Empty => {
                    // If global queue is empty, yield to avoid spinning too aggressively
                    thread::yield_now();

                    // Check if we should shut down
                    if SHUTDOWN.load(Ordering::SeqCst) && GLOBAL_QUEUE.is_empty() {
                        break;
                    }

                    // Small backoff to avoid burning CPU
                    thread::sleep(std::time::Duration::from_millis(1));
                }
                Steal::Retry => continue,
            }
        }

        println!("Worker thread {} stopped", id);
    }

    /// Spawn a future onto the executor
    pub fn spawn<F, T>(future: F) -> Task<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (runnable, task) = async_task::spawn(future, |runnable| {
            GLOBAL_QUEUE.push(runnable);
        });

        // Schedule the task immediately
        runnable.schedule();

        task
    }

    /// Block the current thread until the future completes
    pub fn block_on<F, T>(&mut self, future: F) -> T
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // Make sure worker threads are started
        self.start();

        let parker = Arc::new(Parker::default());
        let unparker = parker.clone();

        // Create a oneshot channel to get the result
        let (sender, receiver) = std::sync::mpsc::channel();

        // Wrap the future to store the result
        let wrapped_future = async move {
            let result = future.await;
            let _ = sender.send(result);
            unparker.unpark();
        };

        // Spawn the wrapped future on the executor
        Self::spawn(wrapped_future).detach();

        // Wait for the future to complete
        parker.park();

        // Return the result
        receiver.recv().expect("Failed to receive result")
    }

    /// Clean up the executor and wait for all worker threads to finish
    pub fn shutdown(&mut self) {
        // Signal shutdown
        SHUTDOWN.store(true, Ordering::SeqCst);
        self.running.store(false, Ordering::SeqCst);

        // Wait for all threads to complete
        let handles = std::mem::take(&mut self.handles);
        for handle in handles {
            let _ = handle.join();
        }
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
