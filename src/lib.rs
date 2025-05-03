pub mod io;
mod parker;
pub mod runtime;

use runtime::{executor::Executor, reactor};

#[cfg(feature = "macro")]
pub use rt_macro::{main, test};

pub fn block_on<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T> + 'static,
    T: 'static,
{
    let mut executor = init();
    executor.block_on(future)
}

pub fn init() -> Executor {
    reactor::start();
    Executor::new()
}

#[cfg(test)]
mod tests {
    use crate::{block_on, io};

    #[test]
    fn it_works() {
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
}
