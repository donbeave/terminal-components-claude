//! Oracle tick contract.
//!
//! Production `App::for_scenario` currently seeks a millisecond target.
//! Oracle `--frame N` is an 80 ms tick ordinal: frame 40 is `now_ms = 3200`,
//! never `seek(40)` as 40 ms.

/// Virtual milliseconds advanced by one oracle simulation tick.
pub const TICK_MS: u64 = 80;

/// Oracle fixture epoch: 2026-09-10 08:41:00 local (UTC+7).
pub const ORACLE_EPOCH_SECS: i64 = 1_789_004_460;

/// One admitted runtime tick advances this many simulation milliseconds,
/// even when the delivery cadence is 200 ms.
pub const ORACLE_ADMITTED_TICK_ADVANCE_MS: i64 = 80;

/// Convert an oracle tick ordinal into the millisecond argument that
/// production `App::for_scenario` interprets as a seek target.
#[must_use]
pub const fn tick_to_ms(tick: u64) -> u64 {
    tick.saturating_mul(TICK_MS)
}

/// Oracle tick ordinal implied by a virtual millisecond clock.
#[must_use]
pub fn ms_to_tick(now_ms: i64) -> u64 {
    u64::try_from(now_ms.max(0)).unwrap_or(0) / TICK_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_40_is_3200_ms_not_40_ms() {
        assert_eq!(tick_to_ms(0), 0);
        assert_eq!(tick_to_ms(1), 80);
        assert_eq!(tick_to_ms(40), 3_200);
        assert_ne!(tick_to_ms(40), 40);
        assert_eq!(ms_to_tick(3_200), 40);
        assert_eq!(ORACLE_EPOCH_SECS, 1_789_004_460);
        assert_eq!(ORACLE_ADMITTED_TICK_ADVANCE_MS, 80);
    }
}
