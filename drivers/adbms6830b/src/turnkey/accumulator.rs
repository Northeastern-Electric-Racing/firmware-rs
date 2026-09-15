//! Service helper. Tracks each chip's PEC failure rate and reports where a break has opened.

use embassy_time::{Duration, Instant};
use crate::turnkey::accumulator::PecMask::Uninitialized;

use super::diagnostics::AccumulatorDiagnostics;
use super::service::service_config;
use super::api::ChipState;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum State {
    /// There is no window open. Watching each update for a chip whose reads are failing.
    /// Basically, everything is normal!
    /// 
    /// This is the default state at startup.
    Idle,
    /// Summing PEC outcomes until `until`, at which point the window is evaluated.
    Accumulating { until: Instant },
    /// A break was found at chip `at`, and the Service is trying to apply the split for it.
    /// 
    /// For context, when the accumulator discovers that a split needs to happen, it calls `Api::split_at()`. `Api::split_at()` can fail
    /// due to SPI errors, and when that happens, we don't have a concrete split state (see the docs for that function). To recover from that,
    /// we have to just keep calling `Api::split_at()` until it succeeds.
    /// 
    /// `attempts` counts how many times we have tried calling it so far, and is
    /// bounded by `segment_isospi_max_split_attempts`.
    /// 
    /// `at` is the index we are trying to split at.
    Splitting { at: usize, attempts: usize },
    /// The split for the break at chip `at` was applied. We are now measuring if it worked or not.
    /// 
    /// Recovery worked if none of the chips from `at` onwards are failing anymore. We continuously run windows
    /// to try verifying, either until it succeeds or we exceed `segment_isospi_max_failed_verification_attempts`.
    /// 
    /// `until` is the Instant this verification window is running until.
    /// 
    /// `total_attempts` is the total number of windows we have tried so far. Unlike `failed_attempts`, this is not bounded by anything.
    /// This is typically in sync with `failed_attempts`, but in the case of low traffic, the accumulator will not be able to prove if chips
    /// are failing or not due to a lack of PEC data. So, the accumulator will be in the `Verifying` state indefinitely with `total_attempts` increasing
    /// each cycle. It will only leave `Verifying` once traffic picks back up, in which it will be able to actually determine a result.
    /// 
    /// `failed_attempts` is the total number of windows that have reported a failure. If this exceeds `segment_isospi_max_failed_verification_attempts`, the
    /// recovery state will transition to `Failed`.
    Verifying { at: usize, until: Instant, total_attempts: usize, failed_attempts: usize },
    /// The split at chip `at` was applied and verified. Recovery is done!!!
    /// 
    /// We only have two isoSPI lines so nothing further is detected from here. If after this point another break
    /// occurs then we can't really do anything. It is kind of like a totem of undying but you only have one
    Recovered { at: usize },
    /// Recovery for the break at chip `at` gave up.
    /// 
    /// Either the split could not be applied, or it was applied but the chips past it never
    /// started communicating again.
    Failed { at: usize },
}

/// result of an `.update()` call. Basically just here to
/// instruct the caller if they need to do anything.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UpdateResult {
    /// we are good. caller doesn't need to do anything
    Okay,

    /// uh oh! We have a break to process
    /// 
    /// the caller is supposed to process this (i.e., actually apply the split for it and then report back through
    /// `Accumulator::was_split_applied()`). This is returned on every update while the split is
    /// still being attempted, which makes it so a failed attempt just gets retried on the next one.
    BreakDetected {
        /// Index of the chip at which the break should be set
        break_chip_index: usize 
    },
}

/// Result of a call to `is_chip_failed()`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum IsChipFailedResult {
    /// Chip meets the failure criteria.
    ChipFailed,
    /// Chip meets the okay criteria (i.e., it is good and is communicating normally)
    ChipOkay,
    /// The chip's total attempts for this window are below `segment_isospi_min_attempts_for_fail`.
    /// In other words, chip cannot be reliably determined as `ChipFailed` or `ChipOkay` because the current window
    /// hasn't collected enough PEC data to do so.
    Undetermined,
}

/// Represents the PecMask states.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PecMask {
    /// Default uninitialized state on startup before the first update() runs.
    Uninitialized,
    /// Normal operation. The PEC mask is not active, so the accumulator is accumulating PEC errors as normal.
    NotMasked,
    /// PEC mask is active. Until `until`, the accumulator does not accumulate PEC errors.
    Masked { until: Instant }
}
impl PecMask {
    /// Whether or not this PecMask is `PecMask::Uninitialized`.
    pub const fn is_uninitialized(&self) -> bool { matches!(&self, PecMask::Uninitialized) }
    /// Whether or not this PecMask is `PecMask::Masked`.
    /// 
    /// Note: This doesn't check if `until` is currently expired or not. That is something the Service
    /// updates every time it runs.
    pub const fn is_masked(&self) -> bool { matches!(&self, PecMask::Masked { until: _ }) }
    /// If this PecMask is `PecMask::Masked`, this returns `Some(until)`. Otherwise, this returns `None`.
    pub const fn until(&self) -> Option<Instant> {
        match self {
            PecMask::Masked { until } => Some(*until),
            _ => None,
        }
    }
}

/// Helper struct for Service that tracks/manages the PEC error accumulator state.
///
/// Note: This doesn't impl Copy because this is a state machine
#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Accumulator<const N: usize> {
    state: State,

    /// PEC failures counted for each chip during this window.
    failed: [usize; N],

    /// Total read attempts for this chip during this window. This is just a sum of pec_success + pec_failed.
    attempts: [usize; N],

    /// Each chip's `pec_failed_count` as of the last update.
    /// 
    /// This is used because what gets accumulated is the difference in errors
    /// between updates.
    last_failed: [usize; N],

    /// Each chip's total PEC outcomes as of the last update. Paired with `last_failed`.
    last_attempts: [usize; N],

    /// Diagnostic counter. 
    /// This counts the number of times a chip has passed over opening a window because
    /// it hadn't taken part in `SEGMENT_ISOSPI_MIN_ATTEMPTS_TO_OPEN_WINDOW` reads since the last time the Service has run.
    /// 
    /// When this increments, it means the number of reads within a single Service cycle is below `SEGMENT_ISOSPI_MIN_ATTEMPTS_TO_OPEN_WINDOW`. At
    /// such a frequency, the Service does not ever open any accumulator windows, because there is not enough PEC error data
    /// to reliably determine whether or not to open a window. Basically, the Service's isoSPI error detection does not run in that cycle.
    /// 
    /// This DOES NOT necessarily indicate an error. In periods of low traffic (where the application is not making any
    /// reads, meaning this Service is effectively the only thing making reads at all), this counter will increment. This is expected,
    /// as in periods where very few reads are being made, there is not enough data to base the PEC error accumulator off of. However, if this
    /// counter is incrementing in normal-to-high traffic periods (i.e., the application is making a lot of reads), there is probably an issue
    /// worth looking into.
    below_min_attempts_to_open_window_count: usize,

    /// Diagnostic counter.
    /// This counts the number of times a chip could not be judged failed or not because it
    /// hadn't taken part in `SEGMENT_ISOSPI_MIN_ATTEMPTS_FOR_FAIL` reads over the whole window.
    /// 
    /// This is only checked when a window expires (i.e., at most once per `SEGMENT_ISOSPI_EVAL_PERIOD_MS`) instead of
    /// once per Service cycle. When it increments, a window did open (so something looked weird enough to open one up)
    /// but ran its full length without gathering enough PEC data to reach a verdict. This basically makes it so the
    /// window gets thrown away at the end.
    /// 
    /// Like `below_min_attempts_to_open_window_count`/`below_min_attempts_to_open_window_count()`, this consistently incrementing
    /// means that isoSPI break detection is basically not running since there are not enough reads being made to gather
    /// the necessary PEC data to detect anything.
    /// 
    /// Also like `below_min_attempts_to_open_window_count`/`below_min_attempts_to_open_window_count()`,
    /// this doesn't necessarily indicate an error. However, if this counter is incrementing when you are in a normal-to-high traffic
    /// period (or at least expect to be), then it is probably good to investigate. 
    /// 
    /// (Plus, remember that this can only increment when a window has been opened, meaning the Service is getting past the 
    /// check reported by `below_min_attempts_to_open_window_count`/`below_min_attempts_to_open_window_count()`. So if this is incrementing,
    /// there is a chance the configured threshold for opening a window may not be optimized that well. Again, it is up to the application
    /// to decide that though, or if they even care about this.)
    below_min_attempts_for_fail_count: usize,

    /// Whether `last_failed` and `last_attempts` hold a real reading yet.
    /// 
    /// This is used to make sure the first reading is saved as a baseline rather than
    /// trying to calculate a change from it.
    seeded: bool,

    /// Service config for the service.
    config: service_config::ServiceConfig,

    /// This is basically the same thing as the PEC mask from the C driver/TSECU-Shep. See enum [PecMask] docs.
    pec_mask: PecMask,

    /// This counts the number of times `update_chips()` (and therefore `update()` itself) has run while a PEC
    /// mask is active. This can be useful to check if chips keep sleeping unexpectedly, and if that sleeping
    /// is stopping the PEC accumulator from progressing.
    /// 
    /// This will increment a few times (depending on your configured Service frequency and PEC mask duration) at startup, and then
    /// whenever chips sleep (if any do) thereafter.
    updates_while_masked_count: usize,
}
impl<const N: usize> Accumulator<N> {
    /// Default initialization for the accumulator. Will be idle by default.
    pub(crate) const fn new(config: service_config::ServiceConfig) -> Self {
        Self {
            state: State::Idle,
            failed: [0; N],
            attempts: [0; N],
            last_failed: [0; N],
            last_attempts: [0; N],
            below_min_attempts_to_open_window_count: 0,
            below_min_attempts_for_fail_count: 0,
            seeded: false,
            config,
            pec_mask: Uninitialized,
            updates_while_masked_count: 0,
        }
    }

    /// PRIVATE! Helper that zeroes this window's tallies for each chip.
    fn reset_chips(&mut self) {
        self.failed = [0; N];
        self.attempts = [0; N];
    }

    /// PRIVATE! Adds each chip's PEC outcomes since the last update to this window.
    fn update_chips(&mut self, chips: &[ChipState; N]) {
        // if mask is uninitialized then this is the first update() that has run. so, we need to initialize PecMask
        if self.pec_mask.is_uninitialized() {
            self.set_masked();
        }

        // if the mask is expired, set it to NotMasked before doing anything
        if let Some(until) = self.pec_mask.until()
            && Instant::now() >= until {
                self.pec_mask = PecMask::NotMasked;
            }

        for (chip, state) in chips.iter().enumerate() {
            let failed = state.pec_failed_count();
            let succeeded = state.pec_success_count();
            let attempts = failed + succeeded;

            if self.seeded && !self.pec_mask.is_masked() {
                self.failed[chip] += failed - self.last_failed[chip];
                self.attempts[chip] += attempts - self.last_attempts[chip];
            }

            self.last_failed[chip] = failed;
            self.last_attempts[chip] = attempts;
        }

        if self.pec_mask.is_masked() {
            self.updates_while_masked_count += 1;
        }

        self.seeded = true;
    }

    /// PRIVATE! This chip's PEC failure rate over the current window, as a percentage.
    /// 
    /// Note: This doesn't check `SEGMENT_ISOSPI_ARM_MIN_ATTEMPTS` or `SEGMENT_ISOSPI_MIN_ATTEMPTS`. The caller
    /// is supposed to do that themselves.
    fn failure_pct(&self, chip: usize) -> u8 {
        // if no reads have happened yet we technically have a failure pct of 0%
        if self.attempts[chip] == 0 {
            return 0;
        }

        ((self.failed[chip] * 100) / self.attempts[chip]) as u8
    }

    /// PRIVATE! Opens an accumulation window.
    /// 
    /// this purposefully keeps whatever is already tallied so the update that armed the
    /// window counts towards it.
    fn open_window(&mut self) {
        self.state = State::Accumulating {
            until: Instant::now() + Duration::from_millis(self.config.segment_isospi_eval_period_ms),
        };
    }

    /// PRIVATE! Helper for `update()`. Checks whether or not `chip`'s PEC data warrants opening a window
    fn should_we_open_window_for_chip(&mut self, failure_pct: &[u8; N], chip: usize) -> bool {
        // first check if we have crossed the minimum amount of PEC attempts to even consider opening a window
        if self.attempts[chip] < self.config.segment_isospi_min_attempts_to_open_window {
            // increment diagnostic for how many times we have not opened a window just due to the min attempts
            self.below_min_attempts_to_open_window_count += 1;
            return false;
        }
        // okay at this point we know we have enough PEC attempt data to actually do the check reliably. So:
        // if this is true we should open a window (since we have exceeded the threshold for failing chips)
        failure_pct[chip] >= self.config.segment_isospi_pec_failure_ratio_pct
    }

    /// PRIVATE! Helper for `update()`. Checks whether or not `chip`'s PEC data passes our failure criteria
    fn is_chip_failed(&mut self, failure_pct: &[u8; N], chip: usize) -> IsChipFailedResult {
        // first check if we have crossed the minimum amount of PEC attempts to even consider the chip as failed
        // if this is too low we don't have enough PEC data to reliably conclude that a chip has failed
        if self.attempts[chip] < self.config.segment_isospi_min_attempts_for_fail {
            // increment diagnostic for how many times a chip couldn't be judged just due to the min attempts
            self.below_min_attempts_for_fail_count += 1;
            return IsChipFailedResult::Undetermined;
        }
        // okay at this point we know we have enough PEC attempt data to actually do the check reliably. So:
        // if this is true we should flag this chip as failed (since we have exceeded the threshold for failing chips)
        if failure_pct[chip] >= self.config.segment_isospi_pec_failure_ratio_pct {
            IsChipFailedResult::ChipFailed
        } else {
            IsChipFailedResult::ChipOkay
        }
    }

    /// CRATE PRIVATE! Reports back whether the split asked for by `UpdateResult::BreakDetected` was applied.
    /// 
    /// On success the accumulator starts measuring whether the split actually fixed anything. On
    /// failure it will ask for the split again on the next update, up to
    /// `segment_isospi_max_split_attempts` times, because a failed `Api::split_at()` leaves the
    /// split state unverified and retrying is the only way to get back to something trustworthy.
    /// 
    /// Calling this in any state other than `Splitting` does nothing.
    pub(crate) fn was_split_applied(&mut self, applied: bool) {
        let State::Splitting { at, attempts } = self.state else {
            return;
        };

        if applied {
            // Only what happens from here on says anything about whether the split worked.
            self.reset_chips();
            self.state = State::Verifying {
                at,
                until: Instant::now()
                    + Duration::from_millis(self.config.segment_isospi_eval_period_ms),
                total_attempts: 0,
                failed_attempts: 0,
            };
        } else if attempts + 1 >= self.config.segment_isospi_max_split_attempts {
            self.state = State::Failed { at };
        } else {
            self.state = State::Splitting { at, attempts: attempts + 1 };
        }
    }

    /// CRATE PRIVATE! This sets activates the PEC mask from `Instant::now()` until `Instant::now() + segment_isospi_recovery_startup_time_ms`. 
    pub(crate) fn set_masked(&mut self) {
        self.reset_chips(); // need to reset error sums since whatever is already accumulated was measured across the sleep
        self.pec_mask = PecMask::Masked { until: Instant::now() + Duration::from_millis(self.config.segment_isospi_recovery_startup_time_ms) }
    }

    /// Updates the PEC error accumulator state, and detects if a break should be set.
    /// 
    /// This should be called in every iteration of the Service runner. This will return either `UpdateResult::Okay`, meaning that
    /// the service runner doesn't need to do anything regarding a break right now, or `UpdateResult::BreakDetected`, after which the
    /// service runner must handle the break accordingly.
    /// 
    /// This also returns a `AccumulatorDiagnostics` for the current update.
    pub(crate) fn update(&mut self, chips: &[ChipState; N]) -> (UpdateResult, AccumulatorDiagnostics<N>) {
        // update the accumulator's tracking of all the chip metadata!
        // need to do this at the beginning of update always
        self.update_chips(chips);

        // calculate failure_pct for all chips
        // this is used both for diagnostics and also how to progress
        // accumulator state
        let mut failure_pct: [u8; N] = [0; N];
        for chip in 0..N {
            failure_pct[chip] = self.failure_pct(chip)
        }

        // these are snapshotted for the diagnostics before the state handling below so they are consistent with failure_pct
        // (the state handler resets these in some cases so if we were to snapshot them at the end they would not be correct)
        let failed = self.failed;
        let attempts = self.attempts;
        let old_state = self.state;

        // actually do the update/state handling stuff
        let update_result = 'update_result: {
            match self.state {
                // case: no window is open, so the tallies hold only this update's reads (they get
                // cleared every update we stay here).
                State::Idle => {
                    if (0..N).any(|chip| { self.should_we_open_window_for_chip(&failure_pct, chip) }) {
                        self.open_window();
                    } else {
                        self.reset_chips();
                    }
                }

                // case: the window is up, so see whether what it caught looks like a break.
                State::Accumulating { until } if Instant::now() >= until => {
                    // find the first chip that exceeds our failure criteria
                    // A break will make all the chips after it unreachable, so the first unreachable chip only
                    // indicates a break if every chip after it is also unreachable
                    let first = (0..N).find(|&chip| { matches!(self.is_chip_failed(&failure_pct, chip), IsChipFailedResult::ChipFailed) });

                    if let Some(idx) = first {
                        // okay we found the `first` failing chip, so now we have to check if all the ones after `first`
                        // are also failing. if they are, this is a break
                        if (idx+1..N).all(|chip| matches!(self.is_chip_failed(&failure_pct, chip), IsChipFailedResult::ChipFailed)) {
                            self.reset_chips();
                            self.state = State::Splitting { at: idx, attempts: 0 };
                            break 'update_result UpdateResult::BreakDetected { break_chip_index: idx };
                        }
                    }

                    // Whatever the window caught wasn't a break, so go back to watching for
                    // a fresh spike rather than measuring straight through.
                    self.reset_chips();
                    self.state = State::Idle;
                }

                // case: we are still in the middle of filling the window so don't need to do anything rn
                State::Accumulating { .. } => {}

                // case: the split for this break hasnt been applied yet, so ask for it again. Anything measured
                // while the split is actively being applied is slop so dont accumulate it
                State::Splitting { at, .. } => {
                    self.reset_chips();
                    break 'update_result UpdateResult::BreakDetected { break_chip_index: at };
                }

                // case: the verification window is up! see whether the chips past the split
                // actually came back/can be communicated with now
                State::Verifying { at, until, total_attempts, failed_attempts } if Instant::now() >= until => {
                    enum Outcome {
                        /// At least one chip was undetermined, so we can't conclude anything right now.
                        AnyUndetermined,
                        /// No chips were undetermined, so we have an `all_okay` result to inspect.
                        /// 
                        /// If `all_okay` is true, all chips came back successful. If `all_okay` is false, at least
                        /// one chip came back failed. 
                        NoneUndetermined{ all_okay: bool},
                    }
                    let outcome = 'get_outcome: {
                        let mut num_failed: usize = 0;
                        for chip in at..N {
                            match self.is_chip_failed(&failure_pct, chip) {
                                IsChipFailedResult::Undetermined => {
                                    break 'get_outcome Outcome::AnyUndetermined;
                                },
                                IsChipFailedResult::ChipFailed => {
                                    num_failed += 1;
                                }
                                IsChipFailedResult::ChipOkay => {}
                            }
                        }
                        // if we get here we have looped through all chips and know none are undetermined
                        // `all_okay` is true only if num_failed == 0
                        break 'get_outcome Outcome::NoneUndetermined { all_okay: num_failed == 0 }
                    };

                    self.state = match outcome {
                        // case: at least one chip as undetermined so we need to retry until we can prove something
                        Outcome::AnyUndetermined => {
                            self.reset_chips();
                            State::Verifying {
                                at,
                                until: Instant::now() + Duration::from_millis(self.config.segment_isospi_eval_period_ms),
                                total_attempts: total_attempts + 1,
                                failed_attempts // keep the same this undetermined doesnt count as failed
                            }
                        },

                        // case: all chips are not undetermined
                        Outcome::NoneUndetermined { all_okay } => {
                            match all_okay {
                                // case: all chips are confirmed okay and we have recovered
                                true => {
                                    self.reset_chips();
                                    State::Recovered { at }
                                },

                                // case: at least one chip failed so we need to either retry, or if the max failed attempts has been exceeded, transition to the failed state
                                false => {
                                    if failed_attempts + 1 >= self.config.segment_isospi_max_failed_verification_attempts {
                                        // if we've exceeded the max failed attempts then its over
                                        self.reset_chips();
                                        State::Failed { at }
                                    } else {
                                        // if we haven't then we can retry
                                        self.reset_chips();
                                        State::Verifying {
                                            at,
                                            until: Instant::now() + Duration::from_millis(self.config.segment_isospi_eval_period_ms),
                                            total_attempts: total_attempts + 1,
                                            failed_attempts: failed_attempts + 1,
                                        }
                                    }
                                }
                            }
                        }
                    };
                }

                // case: still filling the verification window so nothing to do yet
                State::Verifying { .. } => {}

                // case: recovery is over, no matter if it was successful or failed. we can't do anything more from here
                State::Recovered { .. } | State::Failed { .. } => self.reset_chips(),
            }

            // if we get here we are okay
            UpdateResult::Okay
        };

        // create the diagnostics
        let diagnostics = AccumulatorDiagnostics {
            previous_state: old_state,
            state: self.state,
            failed,
            attempts,
            failure_pct,
            failure_pct_threshold: self.config.segment_isospi_pec_failure_ratio_pct,
            accumulator_window_period: self.config.segment_isospi_eval_period_ms,
            min_attempts_for_fail: self.config.segment_isospi_min_attempts_for_fail,
            below_min_attempts_to_open_window_count: self.below_min_attempts_to_open_window_count,
            below_min_attempts_for_fail_count: self.below_min_attempts_for_fail_count,
            min_attempts_to_open_window: self.config.segment_isospi_min_attempts_to_open_window,
            pec_mask: self.pec_mask,
            recovery_startup_time: self.config.segment_isospi_recovery_startup_time_ms,
            updates_while_masked_count: self.updates_while_masked_count,
        };

        (update_result, diagnostics)
    }
}