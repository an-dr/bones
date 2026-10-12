use std::panic::AssertUnwindSafe;
use std::sync::{mpsc, OnceLock};

/// Keeps SDL and GTK on one owning thread across libtest test cases.
/// Panics return to libtest without killing the shared thread.
pub(super) fn run(body: impl FnOnce() + Send + 'static) {
    static JOBS: OnceLock<mpsc::Sender<Box<dyn FnOnce() + Send>>> = OnceLock::new();
    let jobs = JOBS.get_or_init(|| {
        let (jobs, queue) = mpsc::channel::<Box<dyn FnOnce() + Send>>();
        std::thread::Builder::new()
            .name("bones-web-sdl".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                for job in queue {
                    job();
                }
            })
            .expect("the SDL thread spawns");
        jobs
    });
    let (done, wait) = mpsc::channel();
    jobs.send(Box::new(move || {
        let _ = done.send(std::panic::catch_unwind(AssertUnwindSafe(body)));
    }))
    .expect("the SDL thread outlives every test");
    if let Err(panic) = wait.recv().expect("the SDL thread reports an outcome") {
        std::panic::resume_unwind(panic);
    }
}
