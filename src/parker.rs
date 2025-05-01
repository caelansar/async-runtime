use std::sync::{Condvar, Mutex};

#[derive(Default)]
pub struct Parker(Mutex<bool>, Condvar);

impl Parker {
    pub fn park(&self) {
        // Acquire a lock to the Mutex which protects our flag indicating if we
        // should resume execution or not.
        let mut resumable = self.0.lock().unwrap();

        // Put this in a loop since there is a chance we'll get woken, but
        // our flag hasn't changed. If that happens, we simply go back to sleep.
        while !*resumable {
            // Wait until notify signal
            resumable = self.1.wait(resumable).unwrap();
        }

        *resumable = false;
    }

    pub fn unpark(&self) {
        *self.0.lock().unwrap() = true;

        // Notify `Condvar` so it wakes up and resumes.
        self.1.notify_one();
    }
}

#[test]
fn parker_works() {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread, time,
    };

    let flag = Arc::new(AtomicBool::new(false));
    let parker = Arc::new(Parker::default());

    let flag_clone = flag.clone();
    let parker_clone = parker.clone();

    thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(200));
        flag_clone.store(true, Ordering::Relaxed);
        parker_clone.unpark();
    });
    assert!(
        !flag.load(Ordering::Relaxed),
        "Flag should be false at this point!"
    );
    parker.park();
    assert!(
        flag.load(Ordering::Relaxed),
        "Flag should be true at this point!"
    );
}
