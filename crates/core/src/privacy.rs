// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// Describes a future outbound operation before any bytes can leave Unfold.
/// There is intentionally no implicit/default authorization path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboundIntent {
    pub destination: String,
    pub asset_count: u64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyStatus {
    pub mode: String,
    pub network_enabled: bool,
    pub telemetry_enabled: bool,
    pub message: String,
}

/// Central deny-by-default policy for future bridges.
///
/// The MVP contains no HTTP client at all. When a bridge is introduced, it
/// must accept this guard and request an explicit, single-use authorization
/// generated from a user-visible intent.
#[derive(Debug, Default)]
pub struct NetworkGuard;

impl NetworkGuard {
    pub fn status(&self) -> PrivacyStatus {
        PrivacyStatus {
            mode: "local_only".into(),
            network_enabled: false,
            telemetry_enabled: false,
            message: "Files and metadata stay on this device.".into(),
        }
    }

    pub fn authorize(&self, _intent: &OutboundIntent) -> Result<(), &'static str> {
        Err("outbound connections are not available in this release")
    }
}

/// Extension point for an eventual Amara Memory handoff. Implementations must
/// remain absent until the product has an explicit review-and-consent flow.
pub trait AmaraMemoryBridge: Send + Sync {
    fn send(&self, intent: &OutboundIntent) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_is_denied_by_default() {
        let guard = NetworkGuard;
        assert!(!guard.status().network_enabled);
        assert!(guard
            .authorize(&OutboundIntent {
                destination: "example.invalid".into(),
                asset_count: 1,
                description: "test".into(),
            })
            .is_err());
    }
}
