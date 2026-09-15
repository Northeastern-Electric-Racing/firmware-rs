//! Service helper. Structs for the diagnostics collected and reported by the Service.

use super::accumulator;
use super::api::{ 
    LineId, ChipState, OnLineA
};
use embassy_time::{ Duration };
use super::super::line::Error;
pub use super::accumulator::PecMask;

/// Diagnostics from the Service's PEC error accumulator.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AccumulatorDiagnostics<const N: usize> {
    /// State of the accumulator accumulator prior to this Service cycle.
    /// 
    /// This is the state `failed`, `attempts`, and `failure_pct` were gathered under.
    pub(crate) previous_state: accumulator::State,
    /// Current state of the accumulator window.
    /// 
    /// This is the state resulting from the `failed`, `attempts`, and `failure_pct` values.
    pub(crate) state: accumulator::State,
    /// Total PEC failures counted for each chip during this window.
    pub(crate) failed: [usize; N],
    /// Total read attempts for this chip during this window.
    /// 
    /// This is just a sum of the number of failed PECs and number of successful PECs for this chip
    /// during this window.
    pub(crate) attempts: [usize; N],
    /// Each chip's PEC failure rate over the current window as a percentage (0 - 100).
    pub(crate) failure_pct: [u8; N],
    /// Percentage of reads that must fail their PEC for a chip to be considered as "failing".
    /// 
    /// Note: This is a constant value! It is literally just an echo of
    /// `segment_isospi_pec_failure_ratio_pct` from the config. It's included in these diagnostics
    /// just for convenience, and so we can double-check this configured constant
    /// is in fact what we expect it to be.
    pub(crate) failure_pct_threshold: u8,
    /// How long the accumulator evaluation window lasts, in ms. Basically, after a window
    /// opens, this is how long the window stays open to gather PEC data.
    /// 
    /// Note: This is a constant value! It is literally just an echo of
    /// `segment_isospi_eval_period_ms` from the config. It's included in these diagnostics
    /// just for convenience, and so we can double-check this configured constant
    /// is in fact what we expect it to be.
    pub(crate) accumulator_window_period: u64,
    /// Fewest reads a chip must have taken part in before its failure rate is actually considered as meaning anything.
    /// 
    /// This is used to protect against a tiny sample size in the accumulator incorrectly flagging a break. Basically,
    /// if an accumulation window is less than this, there is not enough data to conclude that a PCT of failed PECs actually
    /// indicates a break. So, if a window has less than this, we ignore that window.
    /// 
    /// Note: This is a constant value! It is literally just an echo of
    /// `segment_isospi_min_attempts_for_fail` from the config. It's included in these diagnostics
    /// just for convenience, and so we can double-check this configured constant
    /// is in fact what we expect it to be.
    pub(crate) min_attempts_for_fail: usize,

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
    pub(crate) below_min_attempts_to_open_window_count: usize,

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
    pub(crate) below_min_attempts_for_fail_count: usize,

    /// Fewest reads in a single update before that update's failure rate can open a window.
    /// 
    /// Note: This is a constant value! It is literally just an echo of
    /// `segment_isospi_min_attempts_to_open_window` from the config. It's included in these diagnostics
    /// just for convenience, and so we can double-check this configured constant
    /// is in fact what we expect it to be.
    pub(crate) min_attempts_to_open_window: usize,
    /// Current PEC mask state.
    pub(crate) pec_mask: accumulator::PecMask,
    /// Length of grace period that occurs after startup or after a sleep, where the accumulator doesn't accumulate PEC errors.
    /// 
    /// Note: This is a constant value! It is literally just an echo of
    /// `segment_isospi_recovery_startup_time_ms` from the config. It's included in these diagnostics
    /// just for convenience, and so we can double-check this configured constant
    /// is in fact what we expect it to be.
    pub(crate) recovery_startup_time: u64,

    /// This counts the number of times `update_chips()` (and therefore `update()` itself) has run while a PEC
    /// mask is active. This can be useful to check if chips keep sleeping unexpectedly, and if that sleeping
    /// is stopping the PEC accumulator from progressing.
    /// 
    /// This will increment a few times (depending on your configured Service frequency and PEC mask duration) at startup, and then
    /// whenever chips sleep (if any do) thereafter.
    pub(crate) updates_while_masked_count: usize,
}
impl<const N: usize> AccumulatorDiagnostics<N> {
    /// State of the accumulator accumulator prior to this Service cycle.
    /// 
    /// This is the state `failed`, `attempts`, and `failure_pct` were gathered under.
    pub const fn previous_state(&self) -> accumulator::State { self.previous_state }
    /// Current state of the accumulator window.
    /// 
    /// This is the state resulting from the `failed`, `attempts`, and `failure_pct` values.
    pub const fn state(&self) -> accumulator::State { self.state }
    /// Total PEC failures counted for each chip during this window.
    pub const fn failed(&self) -> [usize; N] { self.failed }
    /// Total read attempts for this chip during this window.
    /// 
    /// This is just a sum of the number of failed PECs and number of successful PECs for this chip
    /// during this window.
    pub const fn attempts(&self) -> [usize; N] { self.attempts }
    /// Each chip's PEC failure rate over the current window as a percentage (0 - 100).
    pub const fn failure_pct(&self) -> [u8; N] { self.failure_pct }
    /// Percentage of reads that must fail their PEC for a chip to be considered as "failing".
    /// 
    /// Note: This is a constant value! It is literally just an echo of
    /// `segment_isospi_pec_failure_ratio_pct` from the config. It's included in these diagnostics
    /// just for convenience, and so we can double-check this configured constant
    /// is in fact what we expect it to be.
    pub const fn failure_pct_threshold(&self) -> u8 { self.failure_pct_threshold }
    /// How long the accumulator evaluation window lasts, in ms. Basically, after a window
    /// opens, this is how long the window stays open to gather PEC data.
    /// 
    /// Note: This is a constant value! It is literally just an echo of
    /// `segment_isospi_eval_period_ms` from the config. It's included in these diagnostics
    /// just for convenience, and so we can double-check this configured constant
    /// is in fact what we expect it to be.
    pub const fn accumulator_window_period(&self) -> u64 { self.accumulator_window_period }
    /// Fewest reads a chip must have taken part in before its failure rate is actually considered as meaning anything.
    /// 
    /// This is used to protect against a tiny sample size in the accumulator incorrectly flagging a break. Basically,
    /// if an accumulation window is less than this, there is not enough data to conclude that a PCT of failed PECs actually
    /// indicates a break. So, if a window has less than this, we ignore that window.
    /// 
    /// Note: This is a constant value! It is literally just an echo of
    /// `segment_isospi_min_attempts_for_fail` from the config. It's included in these diagnostics
    /// just for convenience, and so we can double-check this configured constant
    /// is in fact what we expect it to be.
    pub const fn min_attempts_for_fail(&self) -> usize { self.min_attempts_for_fail }

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
    pub const fn below_min_attempts_to_open_window_count(&self) -> usize { self.below_min_attempts_to_open_window_count }

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
    pub const fn below_min_attempts_for_fail_count(&self) -> usize { self.below_min_attempts_for_fail_count }

    /// Fewest reads in a single update before that update's failure rate can open a window.
    /// 
    /// Note: This is a constant value! It is literally just an echo of
    /// `segment_isospi_min_attempts_to_open_window` from the config. It's included in these diagnostics
    /// just for convenience, and so we can double-check this configured constant
    /// is in fact what we expect it to be.
    pub const fn min_attempts_to_open_window(&self) -> usize { self.min_attempts_to_open_window }

    /// Current PEC mask state.
    pub const fn pec_mask(&self) -> accumulator::PecMask { self.pec_mask }
    /// Length of grace period that occurs after startup or after a sleep, where the accumulator doesn't accumulate PEC errors.
    /// 
    /// Note: This is a constant value! It is literally just an echo of
    /// `segment_isospi_recovery_startup_time_ms` from the config. It's included in these diagnostics
    /// just for convenience, and so we can double-check this configured constant
    /// is in fact what we expect it to be.
    pub const fn recovery_startup_time(&self) -> u64 { self.recovery_startup_time }
    /// This counts the number of times `update_chips()` (and therefore `update()` itself) has run while a PEC
    /// mask is active. This can be useful to check if chips keep sleeping unexpectedly, and if that sleeping
    /// is stopping the PEC accumulator from progressing.
    /// 
    /// This will increment a few times (depending on your configured Service frequency and PEC mask duration) at startup, and then
    /// whenever chips sleep (if any do) thereafter.
    pub const fn updates_while_masked_count(&self) -> usize { self.updates_while_masked_count }
}

/// Diagnostics for the frequency at which the Service is running.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TimingDiagnostics {
    /// The difference in time between the most recent Service cycle, and the Service cycle before that.
    /// 
    /// This should generally be around the frequency at which you run the service, but may be slightly longer.
    /// 
    /// If only zero or one Service cycles have ran yet, this will be None.
    pub(crate) period: Option<Duration>,
    /// The maximum `period` the Service has observed while running.
    /// 
    /// This starts out as zero until the Service has ran a few times.
    pub(crate) max_period: Duration,
    /// How long the "work" of the Service took during the most recent Service cycle.
    /// "work" is defined as everything the Service actually does while running (i.e., sleep detection, isoSPI break detection, etc).
    pub(crate) work: Duration,
    /// The maximum `work` the Service has observed while running.
    /// 
    /// This starts out as zero until the Service has ran a few times.
    pub(crate) max_work: Duration,
}
impl TimingDiagnostics {
    /// The difference in time between the most recent Service cycle, and the Service cycle before that.
    /// 
    /// This should generally be around the frequency at which you run the service, but will be slightly longer.
    /// 
    /// If only zero or one Service cycles have ran yet, this will be None.
    pub const fn period(&self) -> Option<Duration> { self.period }
    /// The maximum `period` the Service has observed while running.
    /// 
    /// This starts out as zero until the Service has ran a few times.
    pub const fn max_period(&self) -> Duration { self.max_period }
    /// How long the "work" of the Service took during the most recent Service cycle.
    /// "work" is defined as everything the Service actually does while running (i.e., sleep detection, isoSPI break detection, etc).
    pub const fn work(&self) -> Duration { self.work }
    /// The maximum `work` the Service has observed while running.
    /// 
    /// This starts out as zero until the Service has ran a few times.
    pub const fn max_work(&self) -> Duration { self.max_work }
}

/// Snapshot of per-chip diagnostics at the time the Service ran. This can help provide insight into
/// the exact state of things at the instant the service ran.
/// 
/// This is generally just a snapshot of the `ChipState` properties for each chip, but with
/// a few omitted values that aren't really relavent as diagnostics (like the cached configs),
/// and a few extras added in due to the extra context Service provides.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChipStateDiagnostics<const N: usize> {
    /// Chip state.
    pub(crate) chip_state: [ChipState; N],
    /// Which line each chip is currently on.
    pub(crate) chip_line: [LineId; N],
}
impl<const N: usize> ChipStateDiagnostics<N> {
    /// Chip state.
    pub const fn chip_state(&self) -> [ChipState; N] { self.chip_state }
    /// Which line each chip is currently on.
    pub const fn chip_line(&self) -> [LineId; N] { self.chip_line }
}

/// Diagnostics related to the two Lines.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LineDiagnostics {
    /// Total number of times Line A has failed with a SPI::Error.
    /// 
    /// Just to note, this refers to raw HAL-level SPI errors. It has nothing to do with PEC errors and other things
    /// manually tracked by the service. Those are reported elsewhere in the diagnostics.
    pub(crate) line_a_error_count: usize,
    /// Most recent `Error` that has occured on Line A. `None` if no errors have occured yet.
    /// 
    /// Just to note, this refers to raw HAL-level SPI errors. It has nothing to do with PEC errors and other things
    /// manually tracked by the service. Those are reported elsewhere in the diagnostics.
    pub(crate) most_recent_line_a_error: Option<Error<embedded_hal_async::spi::ErrorKind>>,
    /// Total number of times Line B has failed with a SPI::Error.
    /// 
    /// Just to note, this refers to raw HAL-level SPI errors. It has nothing to do with PEC errors and other things
    /// manually tracked by the service. Those are reported elsewhere in the diagnostics.
    pub(crate) line_b_error_count: usize,
    /// Most recent `Error` that has occured on Line B. `None` if no errors have occured yet.
    /// 
    /// Just to note, this refers to raw HAL-level SPI errors. It has nothing to do with PEC errors and other things
    /// manually tracked by the service. Those are reported elsewhere in the diagnostics.
    pub(crate) most_recent_line_b_error: Option<Error<embedded_hal_async::spi::ErrorKind>>,
    /// Number of chips detected to be reachable from Line A. This will return and error if the reads was not successful.
    /// 
    /// Note: "REACHABLE" is the main word here. This data comes from the `detect_chips()` function, not from any of the isoSPI recovery
    /// accumulator state logic. This value is an oneshot detection of the chips reachable on a Line, and doesn't reflect anything about the
    /// official line split state. It also may fluctuate between Service cycles, especially if there is noise causing PEC errors.
    pub(crate) line_a_chips_detected_count: Result<usize, Error<embedded_hal_async::spi::ErrorKind>>,
    /// Number of chips detected to be reachable from Line B. This will return an error if the reads was not successful.
    /// 
    /// Note: "REACHABLE" is the main word here. This data comes from the `detect_chips()` function, not from any of the isoSPI recovery
    /// accumulator state logic. This value is an oneshot detection of the chips reachable on a Line, and doesn't reflect anything about the
    /// official line split state. It also may fluctuate between Service cycles, especially if there is noise causing PEC errors.
    pub(crate) line_b_chips_detected_count: Result<usize, Error<embedded_hal_async::spi::ErrorKind>>,
}
impl LineDiagnostics {
    /// Total number of times Line A has failed with a SPI::Error.
    /// 
    /// Just to note, this refers to raw HAL-level SPI errors. It has nothing to do with PEC errors and other things
    /// manually tracked by the service. Those are reported elsewhere in the diagnostics.
    pub const fn line_a_error_count(&self) -> usize { self.line_a_error_count }
    /// Most recent `Error` that has occured on Line A. `None` if no errors have occured yet.
    /// 
    /// Just to note, this refers to raw HAL-level SPI errors. It has nothing to do with PEC errors and other things
    /// manually tracked by the service. Those are reported elsewhere in the diagnostics.
    pub const fn most_recent_line_a_error(&self) ->  Option<Error<embedded_hal_async::spi::ErrorKind>> { self.most_recent_line_a_error }
    /// Total number of times Line B has failed with a SPI::Error.
    /// 
    /// Just to note, this refers to raw HAL-level SPI errors. It has nothing to do with PEC errors and other things
    /// manually tracked by the service. Those are reported elsewhere in the diagnostics.
    pub const fn line_b_error_count(&self) -> usize { self.line_b_error_count }
    /// Most recent `Error` that has occured on Line B. `None` if no errors have occured yet.
    /// 
    /// Just to note, this refers to raw HAL-level SPI errors. It has nothing to do with PEC errors and other things
    /// manually tracked by the service. Those are reported elsewhere in the diagnostics.
    pub const fn most_recent_line_b_error(&self) -> Option<Error<embedded_hal_async::spi::ErrorKind>> { self.most_recent_line_b_error }
    /// Number of chips detected to be reachable from Line A. This will return and error if the reads was not successful.
    /// 
    /// Note: "REACHABLE" is the main word here. This data comes from the `detect_chips()` function, not from any of the isoSPI recovery
    /// accumulator state logic. This value is an oneshot detection of the chips reachable on a Line, and doesn't reflect anything about the
    /// official line split state. It also may fluctuate between Service cycles, especially if there is noise causing PEC errors.
    pub const fn line_a_chips_detected_count(&self) -> Result<usize, Error<embedded_hal_async::spi::ErrorKind>> { self.line_a_chips_detected_count }
    /// Number of chips detected to be reachable from Line B. This will return and error if the reads was not successful.
    /// 
    /// Note: "REACHABLE" is the main word here. This data comes from the `detect_chips()` function, not from any of the isoSPI recovery
    /// accumulator state logic. This value is an oneshot detection of the chips reachable on a Line, and doesn't reflect anything about the
    /// official line split state. It also may fluctuate between Service cycles, especially if there is noise causing PEC errors.
    pub const fn line_b_chips_detected_count(&self) -> Result<usize, Error<embedded_hal_async::spi::ErrorKind>> { self.line_b_chips_detected_count }
}

/// Diagnostics for a Service.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ServiceDiagnostics<const N: usize> {
    /// PEC error accumulator diagnostics.
    pub(crate) accumulator_diagnostics: AccumulatorDiagnostics<N>,
    /// Diagnostics for the frequency at which the Service is running.
    pub(crate) timing_diagnostics: TimingDiagnostics,
    /// Chip state diagnostics.
    pub(crate) chip_state_diagnostics: ChipStateDiagnostics<N>,
    /// /// Diagnostics related to the two Lines.
    pub(crate) line_diagnostics: LineDiagnostics,
    /// Number of times sleep detection has failed due to a SPI::Error.
    pub(crate) sleep_detection_spi_error_count: usize,
    /// Number of times break detection has failed due to a SPI::Error.
    pub(crate) break_detection_spi_error_count: usize,
    /// Number of times the service has ran so far. This increments on every loop the service makes.
    pub(crate) cycles_count: usize,
    /// The current split of chips between the isoSPI lines.
    /// 
    /// This is reported here as the raw OnLineA value. However, the service also
    /// derives the per-chip Line values as part of the per chip diagnostics in case that's easier to read.
    pub(crate) split: OnLineA,
    /// The configured maximum split attempts for isoSPI recovery.
    /// 
    /// This is a const value! It literally is just an echo of `segment_isospi_max_split_attempts` from the config. This
    /// is reported as a diagnostic for convinience, and as a double-check to let you confirm that the config is what you expect
    /// it to be.
    pub(crate) segment_isospi_max_split_attempts: usize,
    /// The configured maximum verification attempts for isoSPI recovery.
    /// 
    /// This is a const value! It literally is just an echo of `segment_isospi_max_failed_verification_attempts` from the config. This
    /// is reported as a diagnostic for convinience, and as a double-check to let you confirm that the config is what you expect
    /// it to be.
    pub(crate) segment_isospi_max_failed_verification_attempts: usize,
    /// Current startup state (associated with the `on_startup` closure).
    pub(crate) startup_reason: super::service::StartupReason,
    /// Most recent startup result (associated with the `on_startup` closure).
    pub(crate) startup_result: super::service::StartupResult,
    /// Number of startups that the Service has ran so far.
    pub(crate) startups_count: usize,
    /// Number of times a startup was requested while a startup of another StartupReason was already active.
    /// 
    /// When this occurs, the Service will still be calling startups just as before, but the StartupReason will be updated.
    pub(crate) startups_overtaken_by_another_startup_counts: usize,
}
impl<const N: usize> ServiceDiagnostics<N> {
    /// Diagnostics from the Service's PEC error accumulator.
    pub const fn accumulator(&self) -> AccumulatorDiagnostics<N> { self.accumulator_diagnostics }
    /// Diagnostics for the frequency at which the Service is running.
    pub const fn timing(&self) -> TimingDiagnostics { self.timing_diagnostics }
    /// Chip state diagnostics.
    pub const fn chip_state_diagnostics(&self) -> ChipStateDiagnostics<N> { self.chip_state_diagnostics }
    /// Diagnostics related to the two Lines.
    pub const fn line_diagnostics(&self) -> LineDiagnostics { self.line_diagnostics }
    /// Number of times sleep detection has failed due to a SPI::Error.
    pub const fn sleep_detection_spi_error_count(&self) -> usize { self.sleep_detection_spi_error_count }
    /// Number of times break detection has failed due to a SPI::Error.
    pub const fn break_detection_spi_error_count(&self) -> usize { self.break_detection_spi_error_count }
    /// Number of times the service has ran so far. This increments on every loop the service makes.
    pub const fn cycles_count(&self) -> usize { self.cycles_count }
    /// The current split of chips between the isoSPI lines.
    /// 
    /// This is reported here as the raw OnLineA value. However, the service also
    /// derives the per-chip Line values as part of the per chip diagnostics in case that's easier to read.
    pub const fn split(&self) -> OnLineA { self.split }
    /// The configured maximum split attempts for isoSPI recovery.
    /// 
    /// This is a const value! It literally is just an echo of `segment_isospi_max_split_attempts` from the config. This
    /// is reported as a diagnostic for convinience, and as a double-check to let you confirm that the config is what you expect
    /// it to be.
    pub const fn max_split_attempts(&self) -> usize { self.segment_isospi_max_split_attempts }
    /// The configured maximum verification attempts for isoSPI recovery.
    /// 
    /// This is a const value! It literally is just an echo of `segment_isospi_max_failed_verification_attempts` from the config. This
    /// is reported as a diagnostic for convinience, and as a double-check to let you confirm that the config is what you expect
    /// it to be.
    pub const fn max_verification_attempts(&self) -> usize { self.segment_isospi_max_failed_verification_attempts }
    /// Current startup reason (associated with the `on_startup` closure).
    pub const fn startup_reason(&self) -> super::service::StartupReason { self.startup_reason }
    /// Most recent startup result (associated with the `on_startup` closure).
    pub const fn startup_result(&self) -> super::service::StartupResult { self.startup_result }
    /// Number of startups that the Service has ran so far.
    pub const fn startups_count(&self) -> usize { self.startups_count }
    /// Number of times a startup was requested while a startup of another StartupReason was already active.
    /// 
    /// When this occurs, the Service will still be calling startups just as before, but the StartupReason will be updated.
    pub const fn startups_overtaken_by_another_startup_counts(&self) -> usize { self.startups_overtaken_by_another_startup_counts }
}
