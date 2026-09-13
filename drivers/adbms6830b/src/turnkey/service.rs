//! Service for ADBMS6830B.

// u_TODO for tomorrow: 
// add a diagnostic for the entire configA register maybe. just to be sure.
// also add detect_num_chips diagnostics for each line just because it would be interesting to see
// maybe even add a .start() flag and a .pause() flag for the service so the sleep recovery stuff can actually be tested, and maybe even low power/sleeo mode could be used if gaf

use embassy_time::{Duration, Instant};
use embedded_hal_async::spi::SpiDevice;
use crate::{
    line::{Error, Line}, turnkey::diagnostics::LineDiagnostics,
};
use super::{
    api::{
        Api, OnLineA,
    },
    diagnostics::{ChipStateDiagnostics, TimingDiagnostics},
    accumulator::{Accumulator, UpdateResult},
};

/// Configuration parameters for the Service. This also includes constant defaults.
pub mod service_config {
    /// Default value for [ServiceConfig::segment_isospi_eval_period_ms].
    pub const SEGMENT_ISOSPI_EVAL_PERIOD_MS: u64 = 4000;
    /// Default value for [ServiceConfig::segment_isospi_min_attempts_for_fail].
    pub const SEGMENT_ISOSPI_MIN_ATTEMPTS_FOR_FAIL: usize = 8;
    /// Default value for [ServiceConfig::segment_isospi_pec_failure_ratio_pct].
    pub const SEGMENT_ISOSPI_PEC_FAILURE_RATIO_PCT: u8 = 75;
    /// Default value for [ServiceConfig::segment_isospi_min_attempts_to_open_window].
    pub const SEGMENT_ISOSPI_MIN_ATTEMPTS_TO_OPEN_WINDOW: usize = 2;
    /// Default value for [ServiceConfig::segment_isospi_max_split_attempts].
    pub const SEGMENT_ISOSPI_MAX_SPLIT_ATTEMPTS: usize = 5;
    /// Default value for [ServiceConfig::segment_isospi_max_failed_verification_attempts].
    pub const SEGMENT_ISOSPI_MAX_FAILED_VERIFICATION_ATTEMPTS: usize = 5;
    /// Default value for [ServiceConfig::segment_isospi_recovery_startup_time_ms].
    pub const SEGMENT_ISOSPI_RECOVERY_STARTUP_TIME_MS: u64 = 1500;

    /// Configuration constants for a Service. Probably just use `ServiceConfig::default()`
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct ServiceConfig {
        /// PEC accumulator setting! How long each evaluation window lasts.
        /// 
        /// This defaults to [SEGMENT_ISOSPI_EVAL_PERIOD_MS] when you use `ServiceConfig::default()`.
        pub segment_isospi_eval_period_ms: u64,
        /// PEC accumulator setting! Fewest reads a chip must have taken part in before its failure rate is actually considered as meaning anything.
        ///
        /// This is here to protect against a tiny sample size in the accumulator incorrectly flagging a break. Basically,
        /// if an accumulation window is less than this, there is not enough data to conclude that a PCT of failed PECs actually
        /// indicates a break. So, if a window has less than this, we ignore that window.
        /// 
        /// This defaults to [SEGMENT_ISOSPI_MIN_ATTEMPTS_FOR_FAIL] when you use `ServiceConfig::default()`.
        /// 
        /// ### WARNING:
        /// You should make sure that, in normal operation, your application isn't reading at a frequency below this setting. If you set this
        /// above the number of reads your application makes during an accumulator window (see `segment_isospi_eval_period_ms`), you will effectively
        /// be disabling the isoSPI recovery routine because you will never reach the minimum number of PEC attempts to declare a failure.
        /// 
        /// As such, it is recommended to set this as low as possible, but above your PEC noise threshold. If this is set too low, you may experience false
        /// positives in regards to break detection due to a low sample size being susceptible to noise. However, if you set this too high, isoSPI recovery may never
        /// be able to occur.
        /// 
        /// Note: If you are unsure of the frequency at which your application makes read attempts during a PEC window, or you want to double-check that
        /// the Service's isoSPI recovery isn't skipping detection due to a read frequency lower than this setting, you can monitor the ServiceDiagnostics data
        /// that gets updated every Service cycle.
        pub segment_isospi_min_attempts_for_fail: usize,
        /// PEC accumulator setting! Percentage of reads that must fail their PEC for a chip to look unreachable.
        /// 
        /// This defaults to [SEGMENT_ISOSPI_PEC_FAILURE_RATIO_PCT] when you use `ServiceConfig::default()`.
        ///
        /// A break will cause the affected chips' reads to fail essentially every
        /// time. So, this value is meant to be quite high. 
        /// This should sit well above any plausible noise level but leaves margin for a link
        /// that is failing intermittently rather than completely.
        pub segment_isospi_pec_failure_ratio_pct: u8,
        /// PEC accumulator setting! Fewest reads in a single update before that update's failure rate can open a window.
        ///
        /// This is kinda meant to take the place of `SEGMENT_ISOSPI_PEC_ACCUM_START_THRESH` from the C code. It serves
        /// a similar-ish function (in that it is a blocker for an accumulator window being allowed to start), but it uses sample
        /// size rather than absolute error count.
        /// 
        /// This defaults to [SEGMENT_ISOSPI_MIN_ATTEMPTS_TO_OPEN_WINDOW] when you use `ServiceConfig::default()`.
        /// 
        /// ### WARNING:
        /// This should be set quite low. If you set this higher than the number of reads your application sends within a service loop, you may end up blocking
        /// the Service from ever opening an isoSPI recovery detection window. So, the safest value is probably something like 1 or 2. It should be set right above the PEC error sum noise level per cycle.
        /// 
        /// Note: If you are unsure of the frequency at which your application makes read attempts during a PEC window, or you want to double-check that
        /// the Service's isoSPI recovery isn't skipping detection due to a read frequency lower than this setting, you can monitor the ServiceDiagnostics data
        /// that gets updated every Service cycle.
        pub segment_isospi_min_attempts_to_open_window: usize,
        /// IsoSPI recovery setting! How many times the Service will try to apply a split before
        /// giving up on recovery.
        /// 
        /// This defaults to [SEGMENT_ISOSPI_MAX_SPLIT_ATTEMPTS] when you use `ServiceConfig::default()`.
        pub segment_isospi_max_split_attempts: usize,
        /// IsoSPI recovery setting! How many evaluation windows the Service will spend checking
        /// whether a split worked before giving up on recovery.
        ///
        /// This is the equivalent of `ISOSPI_RECOVERY_VERIFICATION_READS` from the C code.
        /// 
        /// This defaults to [SEGMENT_ISOSPI_MAX_FAILED_VERIFICATION_ATTEMPTS] when you use `ServiceConfig::default()`.
        pub segment_isospi_max_failed_verification_attempts: usize,
        /// "Grace period" before isoSPI recovery windows can start accumulating, applied after startup and after the Service detects a chip fell asleep.
        /// 
        /// In other words: For `segment_isospi_recovery_startup_time_ms` milliseconds after startup/wakeup, isoSPI error detection will be disabled. This allows the chips
        /// to settle down before we hold them against standards.
        /// 
        /// This defaults to [SEGMENT_ISOSPI_RECOVERY_STARTUP_TIME_MS] when you use `ServiceConfig::default()`.
        pub segment_isospi_recovery_startup_time_ms: u64,
    }
    impl Default for ServiceConfig {
        fn default() -> Self {
            Self {
                segment_isospi_eval_period_ms: SEGMENT_ISOSPI_EVAL_PERIOD_MS,
                segment_isospi_min_attempts_for_fail: SEGMENT_ISOSPI_MIN_ATTEMPTS_FOR_FAIL,
                segment_isospi_pec_failure_ratio_pct: SEGMENT_ISOSPI_PEC_FAILURE_RATIO_PCT,
                segment_isospi_min_attempts_to_open_window: SEGMENT_ISOSPI_MIN_ATTEMPTS_TO_OPEN_WINDOW,
                segment_isospi_max_split_attempts: SEGMENT_ISOSPI_MAX_SPLIT_ATTEMPTS,
                segment_isospi_max_failed_verification_attempts: SEGMENT_ISOSPI_MAX_FAILED_VERIFICATION_ATTEMPTS,
                segment_isospi_recovery_startup_time_ms: SEGMENT_ISOSPI_RECOVERY_STARTUP_TIME_MS,
            }
        }
    }
}

use super::diagnostics::ServiceDiagnostics;

/// State of the Service in regards to startup, corresponding to the `on_startup` closure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum StartupResult {
    /// Startup is not complete yet. The Service will call the `on_startup` closure every cycle
    /// until this transitions to `Complete`.
    /// 
    /// This is the default state at boot time.
    Incomplete,
    /// Startup finished with no errors.
    Complete,
}
impl StartupResult {
    /// Whether or not this is `StartupResult::Incomplete`.
    pub const fn is_incomplete(&self) -> bool { matches!(self, StartupResult::Incomplete) }
}

pub struct Service<SPI: SpiDevice, const N: usize> {
    api: Api<SPI, N>,

    /// The config this service holds. this is never meant to be mutated after
    /// construction time. The fields are all public tho so the user can declare the
    /// config with the nice declarative syntax. Just internally, service.rs isn't supposed to
    /// modify the config after we store it
    service_config: service_config::ServiceConfig,

    accumulator: Accumulator<N>,
    sleep_detection_spi_error_count: usize,
    cycles_count: usize,
    break_detection_spi_error_count: usize,

    /// Current startup state (associated with the `on_startup` closure).
    startup_reason: StartupReason,
    /// Most recent startup result (associated with the `on_startup` closure).
    startup_result: StartupResult,

    /// timestamp the service last ran
    previous_run_timestamp: Option<Instant>,
    /// the highest period between two service runs we have observed so far
    max_period: Duration,
    /// the highest time we have observed the work of the service run taking
    max_work: Duration,
    /// counts the number of times startup has run for diagnostics
    startups_count: usize,
    /// counts how many times a startup for another reason was triggered before the current startup could finish. for diagnostics.
    startups_overtaken_by_another_startup_counts: usize,
}

/// Reason why the Service has invoked `on_startup`.
/// 
/// This is not updated every Service cycle. This is only updated either at boot, when sleep
/// is detected, or when isoSPI recovery occurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum StartupReason {
    /// `on_startup` has been called because we are waking up from sleep, either because we are booting up or beacuse we have detected sleep at runtime.
    /// 
    /// Note that this is triggered if any chip at all is discovered as sleeping during a service cycle. It's technically possible that only one or two chips were sleeping, while
    /// the others were fine.
    /// 
    /// Important: It is the application's job to clear the SLEEP bit in this case. Otherwise, the Service will continue
    /// reporting a detected SLEEP since it has not yet been acknowledged by the application. This can be done by just calling `.reset()` in
    /// the on_startup routine.
    FromSleep,
    /// `on_startup` has been called because an isoSPI break was detected, so all chips are getting re-initialized.
    IsospiBreak,
}
impl StartupReason {
    /// Indicates if this `StartupReason` is `StartupReason::IsospiBreak`.
    pub const fn is_isospi_break(&self) -> bool { matches!(&self, StartupReason::IsospiBreak) }
    /// Indicates if this `StartupReason` is `StartupReason::FromSleep`.
    pub const fn is_from_sleep(&self) -> bool { matches!(&self, StartupReason::FromSleep) }
}

impl<SPI: SpiDevice, const N: usize> Service<SPI, N> {
    /// Creates a new service.
    /// ### Parameters
    /// - `line_a`: `Line` instance representing Line A.
    /// - `line_b`: `Line` instance representing Line B.
    /// - `service_config`: High-level service configuration settings in regards to how it runs.
    pub const fn new(line_a: Line<SPI, N>, line_b: Line<SPI, N>, service_config: service_config::ServiceConfig) -> Self {
        Self {
            api: Api::new(line_a, line_b),
            service_config,
            accumulator: Accumulator::<N>::new(service_config),
            sleep_detection_spi_error_count: 0,
            cycles_count: 0,
            break_detection_spi_error_count: 0,
            startup_reason: StartupReason::FromSleep,
            startup_result: StartupResult::Incomplete,
            previous_run_timestamp: None,
            max_period: Duration::MIN,
            max_work: Duration::MIN,
            startups_count: 0,
            startups_overtaken_by_another_startup_counts: 0,
        }
    }

    /// Runs the Service. This will return the cycle's `ServiceDiagnostics` each time you call it.
    ///
    /// This is meant to be called at a consistent frequency by the application.
    ///
    /// ### Parameters
    /// - `on_startup`: This is a closure that provides an `Api` for a startup routine. You are able to dispatch any commands/configs you want for your startup profile here. It is recommended to set ConfigA/ConfigB here. This closure will
    /// be invoked at boot time, and any time the system needs to be re-initialized following a sleep or isoSPI recovery.
    ///
    /// This closure also provides a `StartupReason`, which tells you why specifically `on_startup` was invoked by the service. This is useful in case you want to have different behavior depending
    /// on the context.
    ///
    /// This closure must return a `StartupResult`. This allows the application to inform the Service of the outcome of the startup logic. If you return `StartupResult::Complete`, the Service will
    /// treat startup as finished, and will not call `on_startup` again unless sleep detection/isoSPI recovery occurs. If you return `StartupResult::Incomplete`, the Service will consider startup as not being finished
    /// yet, and will try calling `on_startup` again on the next cycle. The Service will continue calling `on_startup` on each cycle until it returns `StartupResult::Complete`.
    ///
    /// It's ultimately up to the application to decide what they consider `Complete` versus `Incomplete` startup. Generally, if a SPI error or something came back during startup and your commands weren't actually written, it probably counts as
    /// StartupResult::Incomplete.
    ///
    /// Important note: You are expected to pass the same closure here every call.
    pub async fn run(&mut self, mut on_startup: impl AsyncFnMut(&mut Api<SPI, N>, StartupReason) -> StartupResult) -> ServiceDiagnostics<N> {
        let run_started_timestamp = Instant::now(); // timestamp at the start of run
        let period = self.previous_run_timestamp.map(|prev| run_started_timestamp.saturating_duration_since(prev));
        self.previous_run_timestamp = Some(run_started_timestamp);
        if let Some(period) = period {
            self.max_period = self.max_period.max(period);
        }

        // helper macro for calling on_startup but also increasing the counters and stuff
        macro_rules! call_on_startup {
            ($rsn:expr) => {{
                let rsn = $rsn;
                self.startups_count += 1;
                if self.startup_result.is_incomplete() && rsn != self.startup_reason {
                    self.startups_overtaken_by_another_startup_counts += 1;
                }
                self.startup_reason = rsn;
                on_startup(&mut self.api, self.startup_reason).await
            }};
        }

        // AREA WHERE WE DO THE ACTUAL WORK OF THE SERVICE LOOP

        let work_start_timestamp = embassy_time::Instant::now();

        // if we are still in StartupResult::Incomplete, we need to call on_startup
        if self.startup_result.is_incomplete() {
            // startup_reason is whatever it already is, since this area is reached either on boot when it is the first Service cycle, or after a startup loop has previously been started and just failed last time
            self.startup_result = call_on_startup!(self.startup_reason);
        }

        // this should run first so the sleep detection reads count towards the accumulator update break detection
        match self.handle_sleep_detection().await {
            Ok(result) => match result {
                SleepDetectionResult::SleepDetected => {
                    // sleep was detected so we need to start up a PEC mask
                    self.accumulator.set_masked();
                    // we also must call on_startup
                    self.startup_result = call_on_startup!(StartupReason::FromSleep);
                },
                SleepDetectionResult::SleepNotDetected => {
                    // don't need to do anything since this is normal
                },
            },
            Err(_err) => {
                self.sleep_detection_spi_error_count += 1;
                #[cfg(feature = "defmt")]
                // we need to use `Debug2Format` because `Error<SPI::Error>` only implements `Format` when the
                // SPI error type does, and we can't gaurauntee that the SPI error type will. `Debug` is guaranteed tho
                // since `embedded_hal::spi::Error` requires it.
                defmt::error!("ADBMS6830B: Service: `handle_sleep_detection()` failed with error: {}", defmt::Debug2Format(&_err));
            }
        }

        let chips = *self.api.chips();

        let (update_result, accumulator_diagnostics) = self.accumulator.update(&chips);
        match update_result {
            UpdateResult::BreakDetected { break_chip_index } => {
                let applied = match self.handle_break_detected(break_chip_index).await {
                    Ok(()) => true,
                    Err(_err) => {
                        self.break_detection_spi_error_count += 1;
                        #[cfg(feature = "defmt")]
                        // we need to use `Debug2Format` because `Error<SPI::Error>` only implements `Format` when the
                        // SPI error type does, and we can't gaurauntee that the SPI error type will. `Debug` is guaranteed tho
                        // since `embedded_hal::spi::Error` requires it.
                        defmt::error!("ADBMS6830B: Service: `handle_break_detection()` failed with error: {}", defmt::Debug2Format(&_err));
                        false
                    }
                };
                // report back to the accumulator if the split was successful or not so it know if it needs to keep trying or can move on
                self.accumulator.was_split_applied(applied);

                // no matter what if a break is detected, we have to re-init everything (this is what tsecu-shepherd does)
                // if `applied` from above is `false` this is probably a bit pointless since this will get retried anyway, however this is useful to have just in case
                self.startup_result = call_on_startup!(StartupReason::IsospiBreak);
            },
            UpdateResult::Okay => {},
        }

        use super::api::LineId;
        let line_a_chip_detection = self.api.detect_chips(LineId::A).await.map_err(|err| err.to_kind());
        let line_b_chip_detection = self.api.detect_chips(LineId::B).await.map_err(|err| err.to_kind());

        // END AREA WHERE WE DO THE ACTUAL WORK OF THE SERVICE LOOP

        let work = Instant::now().saturating_duration_since(work_start_timestamp);
        self.max_work = self.max_work.max(work);

        // this is counted before we update the diagnostics so the diagnostics include the cycle being reported!!
        self.cycles_count += 1;
        
        ServiceDiagnostics {
            accumulator_diagnostics,
            timing_diagnostics: TimingDiagnostics {
                period, max_period: self.max_period, work, max_work: self.max_work,
            },
            chip_state_diagnostics: ChipStateDiagnostics {
                // this calls `*api.chips()` again instead of just using the already-read `chips`
                // to make sure the diagnostics gets the absolute final ChipState at the end of the
                // Service cycle
                chip_state: *self.api.chips(),
                chip_line: core::array::from_fn(|i| self.api.line_of(i)),
            },
            line_diagnostics: LineDiagnostics {
                line_a_error_count: self.api.line_a_error_count,
                most_recent_line_a_error: self.api.most_recent_line_a_error,
                line_b_error_count: self.api.line_b_error_count,
                most_recent_line_b_error: self.api.most_recent_line_b_error,
                line_a_chips_detected_count: line_a_chip_detection,
                line_b_chips_detected_count: line_b_chip_detection,
            },
            split: self.api.split(),
            sleep_detection_spi_error_count: self.sleep_detection_spi_error_count,
            break_detection_spi_error_count: self.break_detection_spi_error_count,
            cycles_count: self.cycles_count,
            segment_isospi_max_split_attempts: self.service_config.segment_isospi_max_split_attempts,
            segment_isospi_max_failed_verification_attempts: self.service_config.segment_isospi_max_failed_verification_attempts,
            startup_reason: self.startup_reason,
            startup_result: self.startup_result,
            startups_count: self.startups_count,
            startups_overtaken_by_another_startup_counts: self.startups_overtaken_by_another_startup_counts,
        }
    }

    /// Provides exclusive access to the inner `Api`.
    pub const fn api(&mut self) -> &mut Api<SPI, N> { &mut self.api }
}

/// Private! Contains the results for a `handle_sleep_detection()` call.
enum SleepDetectionResult {
    /// Sleep was detected during this call.
    /// 
    /// At least one chip has appeared to sleep since we last checked.
    SleepDetected,
    /// Sleep was not detected during this call.
    /// 
    /// No chips appeared to sleep since we last checked.
    SleepNotDetected,
}

/// # Helpers
/// 
/// Internal helpers for the service.
impl<SPI: SpiDevice, const N: usize> Service<SPI, N> {
    /// PRIVATE! Logic for when a break has been detected.
    /// 
    /// This should be called when a break is detected. `break_chip_index` should be
    /// passed in here.
    async fn handle_break_detected(&mut self, break_chip_index: usize) -> Result<(), Error<SPI::Error>> {
        let api = &mut self.api;

        match api.split_at(OnLineA(break_chip_index)).await {
            Ok(()) => {
                #[cfg(feature = "defmt")] {
                    defmt::info!(
                        "ADBMS6830B: Service: isoSPI break at chip {}. Chips {}..{} moved to line B.",
                        break_chip_index, break_chip_index, N
                    );
                }
                Ok(())
            },
            Err(err) => {
                #[cfg(feature = "defmt")] {
                    defmt::error!(
                        "ADBMS6830B: Service: failed to split the chain at chip {}: {}",
                        break_chip_index, defmt::Debug2Format(&err)
                    );
                }
                Err(err)
            }
        }
    }

    /// PRIVATE! Detects chips that have slept.
    async fn handle_sleep_detection(&mut self) -> Result<SleepDetectionResult, Error<SPI::Error>> {
        use crate::chip::registers:: {
            status::StatusC,
            status::types::c::SleepModeDetection,
        };

        let api = &mut self.api;

        // note: RDSTATC doesn't increment the command counter (good)
        let mut statuses = api.read::<StatusC>().await;
        if statuses.all_ok() && statuses.iter().flatten().all(|r: crate::line::ChipResponse<StatusC>| r.data().sleep() == SleepModeDetection::SleepModeNotDetected) {
            return Ok(SleepDetectionResult::SleepNotDetected);
        }

        // Something is off, so wake the chain and take a reading of the SLEEP bit
        api.wakeup().await?;
        statuses = api.read::<StatusC>().await;

        for response in statuses.iter() {
            let Some(response) = response else { continue };
            if response.pec().is_failed() { continue; }
            if response.data().sleep() == SleepModeDetection::SleepModeDetected {
                return Ok(SleepDetectionResult::SleepDetected);
            }
        }

        // if we get here then no sleep was detected
        Ok(SleepDetectionResult::SleepNotDetected)
    }
}
