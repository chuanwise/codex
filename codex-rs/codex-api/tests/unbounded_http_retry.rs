use std::sync::Mutex;
use std::time::Duration;

use codex_client::RetryOn;
use codex_client::RetryPolicy;
use codex_client::run_with_retry;
use codex_http_client::Request;
use codex_http_client::RetryAfter;
use codex_http_client::TransportError;
use http::Method;
use http::StatusCode;
use pretty_assertions::assert_eq;
use tokio::time::Instant;

fn policy(max_attempts: u64) -> RetryPolicy {
    RetryPolicy {
        max_attempts,
        base_delay: Duration::from_millis(1),
        retry_on: RetryOn {
            retry_429: false,
            retry_5xx: false,
            retry_transport: true,
        },
    }
}

fn request() -> Request {
    Request::new(Method::POST, "https://example.test/responses".to_string())
}

fn http_error(status: StatusCode, retry_after: Option<RetryAfter>) -> TransportError {
    TransportError::Http {
        status,
        url: None,
        headers: None,
        body: None,
        retry_after,
    }
}

#[tokio::test(start_paused = true)]
async fn every_non_success_status_retries_even_with_zero_budget() {
    for code in 100..=599 {
        let status = StatusCode::from_u16(code).unwrap();
        if status.is_success() {
            continue;
        }
        let result = run_with_retry(
            policy(/*max_attempts*/ 0),
            request,
            |_, attempt| async move {
                if attempt == 0 {
                    Err(http_error(status, None))
                } else {
                    Ok(attempt)
                }
            },
        )
        .await
        .unwrap();
        assert_eq!(result, 1, "HTTP {code}");
    }
}

#[tokio::test(start_paused = true)]
async fn http_retries_continue_past_budget_and_backoff_caps_at_sixty_seconds() {
    let times = Mutex::new(Vec::new());
    let result = run_with_retry(policy(/*max_attempts*/ 1), request, |_, attempt| {
        times.lock().unwrap().push(Instant::now());
        async move {
            if attempt < 8 {
                Err(http_error(StatusCode::SERVICE_UNAVAILABLE, None))
            } else {
                Ok(attempt)
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(result, 8);
    let times = times.into_inner().unwrap();
    let delays: Vec<_> = times
        .windows(2)
        .map(|pair| pair[1].duration_since(pair[0]).as_secs())
        .collect();
    assert_eq!(delays, vec![5, 10, 20, 40, 60, 60, 60, 60]);
}

#[tokio::test(start_paused = true)]
async fn retry_after_cannot_exceed_sixty_seconds() {
    let started = Instant::now();
    run_with_retry(
        policy(/*max_attempts*/ 0),
        request,
        |_, attempt| async move {
            if attempt == 0 {
                Err(http_error(
                    StatusCode::TOO_MANY_REQUESTS,
                    RetryAfter::from_delay(Duration::from_secs(3600)),
                ))
            } else {
                Ok(())
            }
        },
    )
    .await
    .unwrap();
    assert_eq!(started.elapsed().as_secs(), 60);
}

#[tokio::test(start_paused = true)]
async fn http_failures_do_not_consume_transport_retry_budget() {
    let result = run_with_retry(
        policy(/*max_attempts*/ 1),
        request,
        |_, attempt| async move {
            match attempt {
                0 | 1 => Err(http_error(StatusCode::UNAUTHORIZED, None)),
                2 => Err(TransportError::Timeout),
                _ => Ok(attempt),
            }
        },
    )
    .await
    .unwrap();
    assert_eq!(result, 3);
}

#[tokio::test(start_paused = true)]
async fn persistent_http_failures_remain_cancellable() {
    let result = tokio::time::timeout(
        Duration::from_secs(120),
        run_with_retry(policy(/*max_attempts*/ 0), request, |_, _| async {
            Err::<(), _>(http_error(StatusCode::FORBIDDEN, None))
        }),
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test(start_paused = true)]
async fn successful_requests_return_immediately() {
    let started = Instant::now();
    let result = run_with_retry(policy(/*max_attempts*/ 0), request, |_, _| async {
        Ok::<_, TransportError>("completed")
    })
    .await
    .unwrap();
    assert_eq!((result, started.elapsed()), ("completed", Duration::ZERO));
}
