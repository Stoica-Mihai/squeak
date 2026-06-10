//! Opt-in firmware-update check. The ONLY network code in squeak — reached
//! only when the user presses `u`. Queries the Keychron Launcher API for the
//! latest firmware version of the connected device (by vid/pid) so it can be
//! compared against what the device reports.
//!
//! Endpoint (decoded from the Launcher bundle + verified live):
//!   GET https://launcher.keychron.cn/api/merchandise/product/vpId/{vpId}
//!   vpId = vid*65536 + pid ; headers client: launcher, env: PROD
//!   -> data.firmware.lasted.version  (latest)

use std::time::Duration;

use anyhow::{Context, Result};

/// Latest firmware version string for `vid:pid`, per the Keychron API.
pub fn latest_version(vid: u16, pid: u16) -> Result<String> {
    let vpid = (vid as u32) * 65536 + pid as u32;
    let url = format!("https://launcher.keychron.cn/api/merchandise/product/vpId/{vpid}");
    let body: serde_json::Value = ureq::get(&url)
        .set("client", "launcher")
        .set("env", "PROD")
        .set("accept", "application/json")
        .timeout(Duration::from_secs(5))
        .call()?
        .into_json()?;
    body["data"]["firmware"]["lasted"]["version"]
        .as_str()
        .map(str::to_string)
        .context("no firmware.lasted.version in response")
}
