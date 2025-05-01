use std::future::ready;

#[cmoon::test]
async fn macro_works() {
    if ready(42).await != 42 {
        unreachable!()
    }
}
