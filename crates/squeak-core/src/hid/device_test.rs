use super::*;

#[test]
fn poll_slice_is_capped_while_budget_remains() {
    assert_eq!(poll_slice_ms(Duration::ZERO), Some(200));
    assert_eq!(poll_slice_ms(TIMEOUT - Duration::from_millis(500)), Some(200));
    assert_eq!(poll_slice_ms(TIMEOUT - Duration::from_millis(100)), Some(100));
}

#[test]
fn poll_slice_never_returns_zero() {
    // Sub-millisecond remainder still polls once rather than spinning.
    assert_eq!(poll_slice_ms(TIMEOUT - Duration::from_micros(400)), Some(1));
}

#[test]
fn poll_slice_ends_at_and_past_the_deadline() {
    assert_eq!(poll_slice_ms(TIMEOUT), None);
    // Overshoot: the loop condition and the slice used to be two separate
    // clock reads, so this case reached `TIMEOUT - elapsed` and panicked.
    assert_eq!(poll_slice_ms(TIMEOUT + Duration::from_millis(1)), None);
    assert_eq!(poll_slice_ms(Duration::from_secs(3600)), None);
}
