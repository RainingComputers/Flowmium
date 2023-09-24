use std::{future::Future, time::Duration};

pub(crate) async fn with_exp_backoff_retry<T, F>(
    operation: impl Fn() -> F,
    retry_message: &'static str,
    max_retry_count: i32,
) -> Option<T>
where
    F: Future<Output = Option<T>>,
{
    let mut backoff_counter = 500;
    let mut retry_count = 0;

    loop {
        match operation().await {
            Some(some_val) => break Some(some_val),
            None => {
                retry_count += 1;
                backoff_counter *= 2;

                match retry_count > max_retry_count {
                    true => break None,
                    false => {
                        tracing::info!(
                            "{} retrying with backoff for {} milliseconds",
                            retry_message,
                            backoff_counter
                        );

                        tokio::time::sleep(Duration::from_millis(backoff_counter)).await
                    }
                }
            }
        };
    }
}
