use std::future::ready;

#[cmoon::test(worker_threads = 2)]
async fn macro_works() -> Result<(), &'static str> {
    if ready(42).await == 42 {
        Ok(())
    } else {
        Err("wowo")
    }
}
