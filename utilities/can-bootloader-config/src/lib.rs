use serde_json::Value;
use std::{collections::HashSet, env, error::Error, fs, path::PathBuf};

const ECUS_JSON: &str =
    include_str!("../../cangen/cangen-macro/Odyssey-Definitions/can-bootloader/ecus.json");

/// Generate typed CAN IDs and the line bitrate for the selected ECU in OUT_DIR.
pub fn generate(ecu: &str) -> Result<(), Box<dyn Error>> {
    let config: Value = serde_json::from_str(ECUS_JSON)?;
    if config["schema_version"].as_u64() != Some(2) {
        return Err("Unsupported ecus.json schema_version; expected 2".into());
    }
    let can = config["ecus"][ecu]["can"]
        .as_object()
        .ok_or_else(|| format!("Missing CAN configuration for {ecu} in ecus.json"))?;
    let bitrate = config["can_line_bitrate"]
        .as_u64()
        .ok_or("Invalid can_line_bitrate in ecus.json")?;
    if !matches!(bitrate, 125_000 | 250_000 | 500_000 | 1_000_000) {
        return Err("Unsupported can_line_bitrate".into());
    }
    let mut output = format!("pub const CAN_LINE_BITRATE: u32 = {bitrate};\n");
    let mut seen = HashSet::new();
    for (field, constant) in [
        ("request_id", Some("REQUEST_ID")),
        ("response_id", Some("RESPONSE_ID")),
        ("write_data_id", Some("DATA_ID")),
        ("boot_request_id", None),
    ] {
        let value = can
            .get(field)
            .ok_or_else(|| format!("Missing {ecu}.{field}"))?;
        let id = match &value["id"] {
            Value::String(text) => text
                .strip_prefix("0x")
                .or_else(|| text.strip_prefix("0X"))
                .filter(|digits| {
                    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_hexdigit())
                })
                .and_then(|digits| u32::from_str_radix(digits, 16).ok()),
            value => value.as_u64().and_then(|id| u32::try_from(id).ok()),
        }
        .ok_or_else(|| format!("Invalid {ecu}.{field}.id"))?;
        let extended = value["is_ext"]
            .as_bool()
            .ok_or_else(|| format!("Invalid {ecu}.{field}.is_ext"))?;
        if id > if extended { 0x1fff_ffff } else { 0x7ff } {
            return Err(format!("CAN ID out of range: {ecu}.{field}").into());
        }
        if !seen.insert((extended, id)) {
            return Err(format!("Duplicate CAN ID: {ecu}.{field}").into());
        }
        if let Some(constant) = constant {
            let (kind, id_type) = if extended {
                ("Extended", "ExtendedId")
            } else {
                ("Standard", "StandardId")
            };
            output.push_str(&format!(
                "pub const {constant}: embedded_can::Id = match embedded_can::{id_type}::new({id}) {{ Some(id) => embedded_can::Id::{kind}(id), None => panic!(\"Invalid CAN ID\") }};\n"
            ));
        }
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or("Missing OUT_DIR; call from build.rs")?);
    fs::write(out.join("ecu_can.rs"), output)?;
    Ok(())
}
