use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const PROTOCOL_VERSION: u32 = 1;
pub const CLI_VERSION: &str = "0.1.0";

/// Success response for `--version`
#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub protocol: u32,
    pub version: &'static str,
}

/// Capabilities response
#[derive(Debug, Serialize)]
pub struct CapabilitiesResponse {
    pub protocol: u32,
    pub capabilities: Capabilities,
}

#[derive(Debug, Serialize)]
pub struct Capabilities {
    #[serde(rename = "types")]
    pub types: Vec<&'static str>,
    #[serde(rename = "params")]
    pub params: Vec<&'static str>,
}

/// Generate success response
#[derive(Debug, Serialize)]
pub struct GenerateResponse {
    pub protocol: u32,
    pub ok: bool,
    pub output: String,
    pub width: u32,
    pub height: u32,
}

/// Error response written to stderr
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub protocol: u32,
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: &'static str,
    pub message: String,
}

/// Shadow type enum parsed from CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShadowType {
    Drop,
    Contact,
    Curved,
    Long,
    Perspective,
    Inner,
}

impl ShadowType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShadowType::Drop => "drop",
            ShadowType::Contact => "contact",
            ShadowType::Curved => "curved",
            ShadowType::Long => "long",
            ShadowType::Perspective => "perspective",
            ShadowType::Inner => "inner",
        }
    }

    pub fn all() -> &'static [&'static str] {
        &["drop", "contact", "curved", "long", "perspective", "inner"]
    }
}

impl std::str::FromStr for ShadowType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "drop" => Ok(ShadowType::Drop),
            "contact" => Ok(ShadowType::Contact),
            "curved" => Ok(ShadowType::Curved),
            "long" => Ok(ShadowType::Long),
            "perspective" => Ok(ShadowType::Perspective),
            "inner" => Ok(ShadowType::Inner),
            _ => Err(format!("unknown shadow type: {}", s)),
        }
    }
}

/// Output a JSON line to stdout and flush.
pub fn write_stdout<T: Serialize>(value: &T) -> io::Result<()> {
    let mut json = serde_json::to_string(value)?;
    json.push('\n');
    io::stdout().write_all(json.as_bytes())?;
    io::stdout().flush()
}

/// Output a JSON error line to stderr.
pub fn write_stderr_error(code: &'static str, message: impl Into<String>) -> io::Result<()> {
    let resp = ErrorResponse {
        protocol: PROTOCOL_VERSION,
        error: ErrorDetail {
            code,
            message: message.into(),
        },
    };
    let mut json = serde_json::to_string(&resp)?;
    json.push('\n');
    io::stderr().write_all(json.as_bytes())?;
    io::stderr().flush()
}

/// Read entire stdin as bytes.
pub fn read_stdin() -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    Ok(buf)
}

/// Write bytes to stdout (binary image data).
pub fn write_stdout_bytes(data: &[u8]) -> io::Result<()> {
    io::stdout().write_all(data)?;
    io::stdout().flush()
}

pub fn version_response() -> VersionResponse {
    VersionResponse {
        protocol: PROTOCOL_VERSION,
        version: CLI_VERSION,
    }
}

pub fn capabilities_response() -> CapabilitiesResponse {
    CapabilitiesResponse {
        protocol: PROTOCOL_VERSION,
        capabilities: Capabilities {
            types: ShadowType::all().to_vec(),
            params: vec![
                "opacity",
                "offset-x",
                "offset-y",
                "blur",
                "color",
                "scale",
                "angle",
                "edge",
                "curve-depth",
                "fade",
                "light-x",
                "light-y",
                "light-z",
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_response_json() {
        let v = version_response();
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("\"protocol\":1"));
        assert!(json.contains("\"version\":\"0.1.0\""));
    }

    #[test]
    fn test_capabilities_response_json() {
        let c = capabilities_response();
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"types\""));
        assert!(json.contains("drop"));
        assert!(json.contains("inner"));
        assert!(json.contains("\"params\""));
        assert!(json.contains("opacity"));
    }

    #[test]
    fn test_shadow_type_parse() {
        assert_eq!("drop".parse::<ShadowType>().unwrap(), ShadowType::Drop);
        assert_eq!("Contact".parse::<ShadowType>().unwrap(), ShadowType::Contact);
        assert_eq!("INNER".parse::<ShadowType>().unwrap(), ShadowType::Inner);
        assert!("bogus".parse::<ShadowType>().is_err());
    }
}
