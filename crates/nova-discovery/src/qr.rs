use anyhow::{anyhow, Result};
use qrcode::render::svg;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

/// Pairing payload exchanged via QR code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QrPairingPayload {
    pub device_id: Uuid,
    pub device_name: String,
    pub platform: String,
    pub pubkey_hex: String,
    pub fingerprint: String,
    pub noise_static_pubkey_hex: String,
    pub addresses: Vec<SocketAddr>,
    pub port: u16,
    pub pin: String, // 6-digit one-time confirmation PIN
}

impl QrPairingPayload {
    /// Serializes payload to JSON string suitable for QR encoding.
    pub fn to_qr_string(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| anyhow!("QR serialization error: {}", e))
    }

    /// Deserializes payload from QR string.
    pub fn from_qr_string(qr_str: &str) -> Result<Self> {
        serde_json::from_str(qr_str).map_err(|e| anyhow!("Invalid QR pairing data: {}", e))
    }

    /// Generates SVG XML string for rendering QR code in frontend/UI.
    pub fn generate_svg(&self) -> Result<String> {
        let text = self.to_qr_string()?;
        let code = QrCode::new(text.as_bytes())
            .map_err(|e| anyhow!("Failed to generate QR code: {}", e))?;
        let image = code
            .render::<svg::Color>()
            .min_dimensions(256, 256)
            .dark_color(svg::Color("#1e293b"))
            .light_color(svg::Color("#ffffff"))
            .build();
        Ok(image)
    }

    /// Generates compact UTF-8 string preview for terminal logging / display.
    pub fn generate_text_preview(&self) -> Result<String> {
        let text = self.to_qr_string()?;
        let code = QrCode::new(text.as_bytes())
            .map_err(|e| anyhow!("Failed to generate QR code: {}", e))?;
        Ok(code.render::<char>().quiet_zone(false).module_dimensions(2, 1).build())
    }
}

/// Generates an SVG QR code for an arbitrary URL (e.g. Android APK download link).
pub fn generate_url_qr_svg(url: &str) -> Result<String> {
    let code = QrCode::new(url.as_bytes())
        .map_err(|e| anyhow!("Failed to generate QR code: {}", e))?;
    let image = code
        .render::<svg::Color>()
        .min_dimensions(256, 256)
        .dark_color(svg::Color("#0f172a"))
        .light_color(svg::Color("#ffffff"))
        .build();
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_payload_and_svg_generation() -> Result<()> {
        let payload = QrPairingPayload {
            device_id: Uuid::new_v4(),
            device_name: "Ubuntu Workstation".to_string(),
            platform: "linux".to_string(),
            pubkey_hex: hex::encode(vec![1u8; 32]),
            fingerprint: "1234-5678-90ab-cdef".to_string(),
            noise_static_pubkey_hex: hex::encode(vec![2u8; 32]),
            addresses: vec!["192.168.1.50:53418".parse()?],
            port: 53418,
            pin: "847291".to_string(),
        };

        let json_str = payload.to_qr_string()?;
        let parsed = QrPairingPayload::from_qr_string(&json_str)?;
        assert_eq!(payload, parsed);

        let svg = payload.generate_svg()?;
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));

        let download_svg = generate_url_qr_svg("https://github.com/princekjha-dev/Nova/releases/latest/download/nova-android.apk")?;
        assert!(download_svg.contains("<svg"));
        println!("APK_DOWNLOAD_QR_SVG_BEGIN\n{}\nAPK_DOWNLOAD_QR_SVG_END", download_svg);

        Ok(())
    }
}
