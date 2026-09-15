//! CLEAR Register groups!
use bitfield_struct::{bitfield, bitenum};
use adbms6830b_macros::BitfieldEnumDefault;

use super::register_group;
use super::super::commands;

/// Field types relavent to the CLEAR registers.
pub mod types {
    use super::{bitenum, BitfieldEnumDefault};

    /// Whether a CLRFLAG/CLOVUV bit clears its corresponding flag.
    ///
    /// See Table 25 on page 27 and Table 26 on page 28 of the datasheet.
    #[repr(u8)]
    #[bitenum]
    #[derive(BitfieldEnumDefault, Copy, Clone, Debug, PartialEq, Eq, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum ClearAction {
        /// Leave this flag as it is.
        #[default]
        #[fallback]
        DontClear = 0,
        /// Clear this flag back to its non-fault state.
        Clear = 1,
    }

}

/// Clear Flags Command (CLRFLAG). Contains six 1-byte registers (so 6 bytes total).
/// 
/// This is basically a W1C register, so writing `ClearAction::DontClear` won't force the flag to zero or anything.
/// The flag states only change if you specifically write `ClearAction::Clear` to them. Because of this,  all of these flags
/// default to `ClearAction::DontClear` if you create an instance with `ClearFlags::new()` (meaning that by default, writing this command
/// won't change the states of any flags). You can then use the builder functions to configure clears for any of the flags you are interested in.
/// 
/// It may also be useful to see the helpers `ClearFlags::clear_all()` and `ClearFlags::clear_all_csflt()` if you want to clear these flags
/// all at once without having to configure each one manually.
/// 
/// See Table 25 on page 27 of the datasheet.
#[register_group(
    bytes = 6,
    write = Some(commands::clear::clrflag().frame()),
    read = None,
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct ClearFlags {
    /// Clear CS1 Fault. Corresponds to `CL_CS1FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs1flt: types::ClearAction,
    /// Clear CS2 Fault. Corresponds to `CL_CS2FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs2flt: types::ClearAction,
    /// Clear CS3 Fault. Corresponds to `CL_CS3FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs3flt: types::ClearAction,
    /// Clear CS4 Fault. Corresponds to `CL_CS4FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs4flt: types::ClearAction,
    /// Clear CS5 Fault. Corresponds to `CL_CS5FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs5flt: types::ClearAction,
    /// Clear CS6 Fault. Corresponds to `CL_CS6FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs6flt: types::ClearAction,
    /// Clear CS7 Fault. Corresponds to `CL_CS7FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs7flt: types::ClearAction,
    /// Clear CS8 Fault. Corresponds to `CL_CS8FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs8flt: types::ClearAction,
    /// Clear CS9 Fault. Corresponds to `CL_CS9FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs9flt: types::ClearAction,
    /// Clear CS10 Fault. Corresponds to `CL_CS10FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs10flt: types::ClearAction,
    /// Clear CS11 Fault. Corresponds to `CL_CS11FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs11flt: types::ClearAction,
    /// Clear CS12 Fault. Corresponds to `CL_CS12FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs12flt: types::ClearAction,
    /// Clear CS13 Fault. Corresponds to `CL_CS13FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs13flt: types::ClearAction,
    /// Clear CS14 Fault. Corresponds to `CL_CS14FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs14flt: types::ClearAction,
    /// Clear CS15 Fault. Corresponds to `CL_CS15FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs15flt: types::ClearAction,
    /// Clear CS16 Fault. Corresponds to `CL_CS16FLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cs16flt: types::ClearAction,

    /// Empty in the middle from bytes 3 to 4.
    #[bits(16, default = 0)]                                           _empty: u16,

    /// Clear SMED. Corresponds to `CL_SMED`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_smed: types::ClearAction,
    /// Clear SED. Corresponds to `CL_SED`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_sed: types::ClearAction,
    /// Clear CMED. Corresponds to `CL_CMED`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_cmed: types::ClearAction,
    /// Clear CED. Corresponds to `CL_CED`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_ced: types::ClearAction,
    /// Clear VDUV. Corresponds to `CL_VDUV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_vduv: types::ClearAction,
    /// Clear VDOV. Corresponds to `CL_VDOV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_vdov: types::ClearAction,
    /// Clear VAUV. Corresponds to `VAUV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_vauv: types::ClearAction,
    /// Clear VAOV. Corresponds to `CL_VAOV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_vaov: types::ClearAction,
    /// Clear OSCCHK. Corresponds to `CL_OSCCHK`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_oscchk: types::ClearAction,
    /// Clear TMODE. Corresponds to `CL_TMODE`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_tmode: types::ClearAction,
    /// Clear THSD. Corresponds to `CL_THSD`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_thsd: types::ClearAction,
    /// Clear SLEEP. Corresponds to `CL_SLEEP`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_sleep: types::ClearAction,
    /// Clear SPIFLT. Corresponds to `CL_SPIFLT`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_spiflt: types::ClearAction,
    /// empty bit
    #[bits(1, default = 0)]                           _emptybit: u8,
    /// Clear VDE Fault. Corresponds to `CL_VDE`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_vde: types::ClearAction,
    /// Clear VDEL Fault. Corresponds to `CL_VDEL`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_vdel: types::ClearAction,
    
    /// The 2-byte padding to make this 6-byte register group fit into u64
    #[bits(16, default = 0)]                                           _padding: u16,
}
impl ClearFlags {
    /// Creates a new `ClearFlags` where all flags are set to `ClearAction::Clear`.
    pub const fn clear_all() -> Self {
        ClearFlags::new()
            .with_cl_cs1flt(types::ClearAction::Clear)
            .with_cl_cs2flt(types::ClearAction::Clear)
            .with_cl_cs3flt(types::ClearAction::Clear)
            .with_cl_cs4flt(types::ClearAction::Clear)
            .with_cl_cs5flt(types::ClearAction::Clear)
            .with_cl_cs6flt(types::ClearAction::Clear)
            .with_cl_cs7flt(types::ClearAction::Clear)
            .with_cl_cs8flt(types::ClearAction::Clear)
            .with_cl_cs9flt(types::ClearAction::Clear)
            .with_cl_cs10flt(types::ClearAction::Clear)
            .with_cl_cs11flt(types::ClearAction::Clear)
            .with_cl_cs12flt(types::ClearAction::Clear)
            .with_cl_cs13flt(types::ClearAction::Clear)
            .with_cl_cs14flt(types::ClearAction::Clear)
            .with_cl_cs15flt(types::ClearAction::Clear)
            .with_cl_cs16flt(types::ClearAction::Clear)
            .with_cl_smed(types::ClearAction::Clear)
            .with_cl_sed(types::ClearAction::Clear)
            .with_cl_cmed(types::ClearAction::Clear)
            .with_cl_ced(types::ClearAction::Clear)
            .with_cl_vduv(types::ClearAction::Clear)
            .with_cl_vdov(types::ClearAction::Clear)
            .with_cl_vauv(types::ClearAction::Clear)
            .with_cl_vaov(types::ClearAction::Clear)
            .with_cl_oscchk(types::ClearAction::Clear)
            .with_cl_tmode(types::ClearAction::Clear)
            .with_cl_thsd(types::ClearAction::Clear)
            .with_cl_sleep(types::ClearAction::Clear)
            .with_cl_spiflt(types::ClearAction::Clear)
            .with_cl_vde(types::ClearAction::Clear)
            .with_cl_vdel(types::ClearAction::Clear)
    }

    /// Creates a new `ClearFlags` where all 16 `...CSFLT` flags are set to `ClearAction::Clear`.
    /// The other flags aren't modified here, so this can be used with other builder functions if needed.
    pub const fn clear_all_csflt() -> Self {
        ClearFlags::new()
            .with_cl_cs1flt(types::ClearAction::Clear)
            .with_cl_cs2flt(types::ClearAction::Clear)
            .with_cl_cs3flt(types::ClearAction::Clear)
            .with_cl_cs4flt(types::ClearAction::Clear)
            .with_cl_cs5flt(types::ClearAction::Clear)
            .with_cl_cs6flt(types::ClearAction::Clear)
            .with_cl_cs7flt(types::ClearAction::Clear)
            .with_cl_cs8flt(types::ClearAction::Clear)
            .with_cl_cs9flt(types::ClearAction::Clear)
            .with_cl_cs10flt(types::ClearAction::Clear)
            .with_cl_cs11flt(types::ClearAction::Clear)
            .with_cl_cs12flt(types::ClearAction::Clear)
            .with_cl_cs13flt(types::ClearAction::Clear)
            .with_cl_cs14flt(types::ClearAction::Clear)
            .with_cl_cs15flt(types::ClearAction::Clear)
            .with_cl_cs16flt(types::ClearAction::Clear)
    }
}

/// Clear Overvoltage and Undervoltage Command (CLOVUV). Contains six 1-byte registers (so 6 bytes total).
/// 
/// This is basically a W1C register, so writing `ClearAction::DontClear` won't force the flag to zero or anything.
/// The flag states only change if you specifically write `ClearAction::Clear` to them. Because of this,  all of these flags
/// default to `ClearAction::DontClear` if you create an instance with `ClearOvervoltageUndervoltage::new()` (meaning that by default, writing this command
/// won't change the states of any flags). You can then use the builder functions to configure clears for any of the flags you are interested in.
/// 
/// It may also be useful to see the helper `ClearOvervoltageUndervoltage::clear_all()` if you want to clear these flags
/// all at once without having to configure each one manually.
/// 
/// See Table 26 on page 28 of the datasheet.
#[register_group(
    bytes = 6,
    write = Some(commands::clear::clovuv().frame()),
    read = None,
)]
#[bitfield(u64, defmt = cfg(feature = "defmt"))]
pub struct ClearOvervoltageUndervoltage {
    /// Clear C1 Undervoltage. Corresponds to `CL_C1UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c1uv: types::ClearAction,
    /// Clear C1 Overvoltage. Corresponds to `CL_C1OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c1ov: types::ClearAction,

    /// Clear C2 Undervoltage. Corresponds to `CL_C2UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c2uv: types::ClearAction,
    /// Clear C2 Overvoltage. Corresponds to `CL_C2OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c2ov: types::ClearAction,

    /// Clear C3 Undervoltage. Corresponds to `CL_C3UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c3uv: types::ClearAction,
    /// Clear C3 Overvoltage. Corresponds to `CL_C3OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c3ov: types::ClearAction,

    /// Clear C4 Undervoltage. Corresponds to `CL_C4UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c4uv: types::ClearAction,
    /// Clear C4 Overvoltage. Corresponds to `CL_C4OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c4ov: types::ClearAction,

    /// Clear C5 Undervoltage. Corresponds to `CL_C5UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c5uv: types::ClearAction,
    /// Clear C5 Overvoltage. Corresponds to `CL_C5OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c5ov: types::ClearAction,

    /// Clear C6 Undervoltage. Corresponds to `CL_C6UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c6uv: types::ClearAction,
    /// Clear C6 Overvoltage. Corresponds to `CL_C6OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c6ov: types::ClearAction,

    /// Clear C7 Undervoltage. Corresponds to `CL_C7UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c7uv: types::ClearAction,
    /// Clear C7 Overvoltage. Corresponds to `CL_C7OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c7ov: types::ClearAction,

    /// Clear C8 Undervoltage. Corresponds to `CL_C8UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c8uv: types::ClearAction,
    /// Clear C8 Overvoltage. Corresponds to `CL_C8OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c8ov: types::ClearAction,

    /// Clear C9 Undervoltage. Corresponds to `CL_C9UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c9uv: types::ClearAction,
    /// Clear C9 Overvoltage. Corresponds to `CL_C9OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c9ov: types::ClearAction,

    /// Clear C10 Undervoltage. Corresponds to `CL_C10UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c10uv: types::ClearAction,
    /// Clear C10 Overvoltage. Corresponds to `CL_C10OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c10ov: types::ClearAction,

    /// Clear C11 Undervoltage. Corresponds to `CL_C11UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c11uv: types::ClearAction,
    /// Clear C11 Overvoltage. Corresponds to `CL_C11OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c11ov: types::ClearAction,

    /// Clear C12 Undervoltage. Corresponds to `CL_C12UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c12uv: types::ClearAction,
    /// Clear C12 Overvoltage. Corresponds to `CL_C12OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c12ov: types::ClearAction,

    /// Clear C13 Undervoltage. Corresponds to `CL_C13UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c13uv: types::ClearAction,
    /// Clear C13 Overvoltage. Corresponds to `CL_C13OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c13ov: types::ClearAction,

    /// Clear C14 Undervoltage. Corresponds to `CL_C14UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c14uv: types::ClearAction,
    /// Clear C14 Overvoltage. Corresponds to `CL_C14OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c14ov: types::ClearAction,

    /// Clear C15 Undervoltage. Corresponds to `CL_C15UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c15uv: types::ClearAction,
    /// Clear C15 Overvoltage. Corresponds to `CL_C15OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c15ov: types::ClearAction,

    /// Clear C16 Undervoltage. Corresponds to `CL_C16UV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c16uv: types::ClearAction,
    /// Clear C16 Overvoltage. Corresponds to `CL_C16OV`
    #[bits(1, default = types::ClearAction::DEFAULT)] pub cl_c16ov: types::ClearAction,

    /// Empty for STDR4 and STDR5.
    #[bits(16, default = 0)]                                           _empty: u16,
    
    /// The 2-byte padding to make this 6-byte register group fit into u64
    #[bits(16, default = 0)]                                           _padding: u16,
}
impl ClearOvervoltageUndervoltage {
    /// Creates a new `ClearOvervoltageUndervoltage` where all flags are set to `ClearAction::Clear`.
    pub const fn clear_all() -> Self {
        ClearOvervoltageUndervoltage::new()
            .with_cl_c1uv(types::ClearAction::Clear)
            .with_cl_c1ov(types::ClearAction::Clear)

            .with_cl_c2uv(types::ClearAction::Clear)
            .with_cl_c2ov(types::ClearAction::Clear)

            .with_cl_c3uv(types::ClearAction::Clear)
            .with_cl_c3ov(types::ClearAction::Clear)

            .with_cl_c4uv(types::ClearAction::Clear)
            .with_cl_c4ov(types::ClearAction::Clear)

            .with_cl_c5uv(types::ClearAction::Clear)
            .with_cl_c5ov(types::ClearAction::Clear)

            .with_cl_c6uv(types::ClearAction::Clear)
            .with_cl_c6ov(types::ClearAction::Clear)

            .with_cl_c7uv(types::ClearAction::Clear)
            .with_cl_c7ov(types::ClearAction::Clear)

            .with_cl_c8uv(types::ClearAction::Clear)
            .with_cl_c8ov(types::ClearAction::Clear)

            .with_cl_c9uv(types::ClearAction::Clear)
            .with_cl_c9ov(types::ClearAction::Clear)

            .with_cl_c10uv(types::ClearAction::Clear)
            .with_cl_c10ov(types::ClearAction::Clear)

            .with_cl_c11uv(types::ClearAction::Clear)
            .with_cl_c11ov(types::ClearAction::Clear)

            .with_cl_c12uv(types::ClearAction::Clear)
            .with_cl_c12ov(types::ClearAction::Clear)

            .with_cl_c13uv(types::ClearAction::Clear)
            .with_cl_c13ov(types::ClearAction::Clear)

            .with_cl_c14uv(types::ClearAction::Clear)
            .with_cl_c14ov(types::ClearAction::Clear)

            .with_cl_c15uv(types::ClearAction::Clear)
            .with_cl_c15ov(types::ClearAction::Clear)

            .with_cl_c16uv(types::ClearAction::Clear)
            .with_cl_c16ov(types::ClearAction::Clear)
    }
}