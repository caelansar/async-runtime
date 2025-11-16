pub mod io;
mod parker;
pub mod runtime;

use std::future::Future;

use runtime::{executor::Executor, reactor};

#[cfg(feature = "macro")]
pub use rt_macro::{main, test};

/// Block the current thread until the future completes
///
/// This function does not require Send because the future runs on the current thread.
pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    let mut executor = init();
    executor.block_on(future)
}

/// Initialize the runtime with a default executor
pub fn init() -> Executor {
    // Start the reactor
    reactor::start();

    // Create a new executor with default thread count
    let mut executor = Executor::new();

    // Start worker threads
    executor.start();

    executor
}

/// Initialize the runtime with a custom number of threads
pub fn init_with_threads(num_threads: usize) -> Executor {
    // Start the reactor
    reactor::start();

    // Create a custom executor with specified thread count
    let mut executor = Executor::with_threads(num_threads);

    // Start worker threads
    executor.start();

    executor
}

/// Spawn a future onto the executor
pub fn spawn<F, T>(future: F) -> runtime::executor::Task<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    Executor::spawn(future)
}

#[cfg(test)]
mod tests {
    use crate::{block_on, init_with_threads, io, spawn};
    use std::time::Duration;
    use std::{thread, time};

    use std::{
        pin::Pin,
        sync::Arc,
        task::{Context, Poll},
        time::Instant,
    };

    use futures::task::AtomicWaker;

    pub struct DelayAtomic {
        shared: Arc<Shared>,
    }

    struct Shared {
        when: Instant,
        waker: AtomicWaker,
    }

    impl DelayAtomic {
        pub fn new(duration: Duration) -> Self {
            let shared = Shared {
                when: Instant::now() + duration,
                waker: AtomicWaker::new(),
            };
            Self {
                shared: Arc::new(shared),
            }
        }
    }

    impl Future for DelayAtomic {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            self.shared.waker.register(cx.waker());
            let shared = Arc::clone(&self.shared);

            thread::spawn(move || {
                let now = Instant::now();

                if now < shared.when {
                    thread::sleep(shared.when - now);
                }
                shared.waker.wake();
            });

            if Instant::now() >= self.shared.when {
                println!("ready");
                Poll::Ready(())
            } else {
                println!("pending");
                Poll::Pending
            }
        }
    }

    #[test]
    fn http_works() {
        let start = std::time::Instant::now();

        block_on(async {
            let res = io::http::Http::get("http://127.0.0.1:8080/100/hello").await;
            println!("{res}");
            let res = io::http::Http::get("http://127.0.0.1:8080/200/world").await;
            println!("{res}");
        });

        let end = std::time::Instant::now();
        println!("Time taken: {:?}", end.duration_since(start));
    }

    #[test]
    fn http_concurrent_works() {
        // Initialize multi-threaded executor with 4 threads
        let mut executor = init_with_threads(4);

        // Spawn 10 tasks that sleep for different durations
        let task1 = spawn(async {
            let res = io::http::Http::get("http://127.0.0.1:8080/100/hello").await;
            res
        });

        let task2 = spawn(async {
            let res = io::http::Http::get("http://127.0.0.1:8080/200/hello").await;
            res
        });

        let tasks = vec![task1, task2];

        // Wait for all tasks to complete
        let results = executor.block_on(async {
            let mut results = Vec::new();
            for task in tasks {
                results.push(task.await);
            }
            results
        });

        println!("{:?}", results);
    }

    #[test]
    fn concurrent_tasks() {
        // Initialize multi-threaded executor with 4 threads
        let mut executor = init_with_threads(4);

        // Spawn 10 tasks that sleep for different durations
        let tasks = (0..10)
            .map(|i| {
                let task = spawn(async move {
                    println!("Task {i} started on thread {:?}", thread::current().name());
                    DelayAtomic::new(Duration::from_secs(1)).await;
                    println!(
                        "Task {i} completed on thread {:?}",
                        thread::current().name()
                    );
                    i
                });
                task
            })
            .collect::<Vec<_>>();

        // Wait for all tasks to complete
        let results = executor.block_on(async {
            let mut results = Vec::new();
            for task in tasks {
                results.push(task.await);
            }
            results
        });

        println!("All tasks completed: {:?}", results);

        // Verify results
        assert_eq!(results, (0..10).collect::<Vec<_>>());
    }
}
