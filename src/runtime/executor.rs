//! Executor decides who gets time on the CPU to progress and when they get it.
//! The executor must also call Future::poll and advance the state machines to their next state. It's a type of scheduler.
//! The current executor is a single-threaded implementation

use std::{
    cell::RefCell,
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::channel,
    },
    task::{Context, Poll, Wake, Waker},
    thread,
};

use crate::parker::Parker;

type Task = Pin<Box<dyn Future<Output = ()>>>;

thread_local! {
    static CURRENT_EXEC: ExecutorCore = ExecutorCore::default();
}

#[derive(Default)]
struct ExecutorCore {
    tasks: RefCell<HashMap<usize, Task>>,
    ready_queue: Arc<Mutex<Vec<usize>>>,
    next_id: AtomicUsize,
}

pub struct Executor {}

impl Executor {
    pub fn new() -> Self {
        Self {}
    }

    fn pop_ready(&self) -> Option<usize> {
        CURRENT_EXEC.with(|q| q.ready_queue.lock().map(|mut q| q.pop()).unwrap())
    }

    fn get_future(&self, id: usize) -> Option<Task> {
        CURRENT_EXEC.with(|q| q.tasks.borrow_mut().remove(&id))
    }

    fn get_waker(&self, id: usize, parker: Arc<Parker>) -> Arc<MyWaker> {
        Arc::new(MyWaker {
            id,
            parker,
            ready_queue: CURRENT_EXEC.with(|q| q.ready_queue.clone()),
        })
    }

    fn insert_task(&self, id: usize, task: Task) {
        CURRENT_EXEC.with(|q| q.tasks.borrow_mut().insert(id, task));
    }

    fn task_count(&self) -> usize {
        CURRENT_EXEC.with(|q| q.tasks.borrow().len())
    }

    /// Block the thread until the future is ready.
    pub fn block_on<F, T>(&mut self, future: F) -> T
    where
        F: Future<Output = T> + 'static,
        T: 'static,
    {
        // Create a special task ID for our main future
        let main_id = CURRENT_EXEC.with(|e| e.next_id.fetch_add(1, Ordering::Relaxed));

        // We need to keep track of the result
        let (tx, rx) = channel();

        // Wrap the future to capture its result
        let wrapped_future = async move {
            let result = future.await;
            let _ = tx.send(result);
        };

        // Spawn the wrapped future
        CURRENT_EXEC.with(|e| {
            e.tasks
                .borrow_mut()
                .insert(main_id, Box::pin(wrapped_future));
            e.ready_queue.lock().map(|mut q| q.push(main_id)).unwrap();
        });

        let parker = Arc::new(Parker::default());

        loop {
            while let Some(id) = self.pop_ready() {
                println!("pop_ready: {id}");

                let mut future = match self.get_future(id) {
                    Some(f) => f,
                    // guard against false wakeups
                    None => continue,
                };

                let waker: Waker = self.get_waker(id, parker.clone()).into();
                let mut cx = Context::from_waker(&waker);

                match future.as_mut().poll(&mut cx) {
                    Poll::Pending => self.insert_task(id, future),
                    Poll::Ready(_) => continue,
                }
            }

            let task_count = self.task_count();
            let name = thread::current().name().unwrap_or_default().to_string();

            if task_count > 0 {
                println!("{name}: {task_count} pending tasks. Sleep until notified.");
                parker.park();
            } else {
                println!("{name}: All tasks are finished");
                break;
            }
        }

        // Extract the result
        rx.recv().expect("Future did not complete")
    }
}

#[derive(Clone)]
pub struct MyWaker {
    parker: Arc<Parker>,
    id: usize,
    ready_queue: Arc<Mutex<Vec<usize>>>,
}

impl Wake for MyWaker {
    fn wake(self: Arc<Self>) {
        self.ready_queue
            .lock()
            .map(|mut q| q.push(self.id))
            .unwrap();
        self.parker.unpark();
    }
}
