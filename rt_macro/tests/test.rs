use std::future::ready;

#[cmoon::test(worker_threads = 2)]
async fn macro_works() -> Result<(), &'static str> {
    cmoon::spawn(async {
        println!("hello");
    })
    .await;

    if ready(42).await == 42 {
        Ok(())
    } else {
        Err("wowo")
    }
}

#[cmoon::test]
async fn macro_works_with_no_threads() -> Result<(), &'static str> {
    if ready(42).await == 42 {
        Ok(())
    } else {
        Err("wowo")
    }
}

/// Test that block_on works with non-Send futures
///
/// This test demonstrates the key difference between spawn() and block_on():
/// - spawn() requires Send because it sends the future to worker threads
/// - block_on() does NOT require Send because it runs on the current thread
#[cmoon::test(worker_threads = 2)]
async fn test_not_send() {
    use std::rc::Rc;

    // Rc is not Send, so this future is not Send
    let local_data = Rc::new(vec![1, 2, 3, 4, 5]);

    // ❌ This would NOT compile if we used spawn():
    // cmoon::spawn(async {
    //     local_data.iter().sum() // Error: Rc cannot be sent between threads
    // }).await;

    // ✅ But this DOES compile with block_on (via the macro):
    let result = async {
        let sum: i32 = local_data.iter().sum();
        println!("Sum of local data: {}", sum);
        sum
    }
    .await;

    assert_eq!(result, 15);
}
