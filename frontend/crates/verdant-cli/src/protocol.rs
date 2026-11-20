use std::{fmt, io::{self, BufRead, Read}};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL: &str = "verdant-cli/1";
pub const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: String,
    pub request_id: String,
    pub op: String,
    pub params: Value,
}

impl Request {
    pub fn parse(bytes: &[u8]) -> Result<Self, CliError> {
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(CliError::new("RESOURCE_EXHAUSTED", "request exceeds 2 MiB"));
        }
        let request: Self = serde_json::from_slice(bytes)
            .map_err(|error| CliError::new("INVALID_ARGUMENT", error.to_string()))?;
        request.validate()
    }

    pub fn validate(self) -> Result<Self, CliError> {
        if self.protocol != PROTOCOL {
            return Err(CliError::new("VERSION_MISMATCH", format!("expected {PROTOCOL}")));
        }
        if self.request_id.is_empty() || self.request_id.len() > 128
            || self.request_id.chars().any(char::is_control)
        {
            return Err(CliError::new("INVALID_ARGUMENT", "request_id must be 1–128 bytes without control characters"));
        }
        if !self.params.is_object() || self.op.is_empty() || self.op.len() > 96 {
            return Err(CliError::new("INVALID_ARGUMENT", "params must be an object"));
        }
        Ok(self)
    }
}

#[derive(Debug, Serialize)]
pub struct CliError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl CliError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: matches!(code, "PROJECT_BUSY" | "RESOURCE_EXHAUSTED" | "WORKER_FAILED"),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn exit_code(&self) -> u8 {
        match self.code {
            "INVALID_ARGUMENT" | "INVALID_PROJECT" | "DECISION_REQUIRED" | "VERSION_MISMATCH" | "OUTPUT_EXISTS" => 2,
            "UNSUPPORTED_CAPABILITY" | "VOICE_NOT_ADMITTED" | "MISSING_ASSET" | "RESOURCE_EXHAUSTED" => 3,
            "PROJECT_BUSY" | "REVISION_CONFLICT" => 4,
            "CANCELLED" => 130,
            "INTERNAL_ERROR" => 70,
            _ => 5,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CliError {}

#[derive(Serialize)]
pub struct Response<'a> {
    protocol: &'static str,
    request_id: Option<&'a str>,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<CliError>,
    diagnostics: Vec<Value>,
}

impl<'a> Response<'a> {
    pub fn new(request_id: Option<&'a str>, outcome: Result<Value, CliError>) -> Self {
        let (result, error) = match outcome {
            Ok(value) => (Some(value), None),
            Err(error) => (None, Some(error)),
        };
        Self { protocol: PROTOCOL, request_id, ok: error.is_none(), result, error, diagnostics: Vec::new() }
    }

    pub fn exit_code(&self) -> u8 {
        self.error.as_ref().map_or(0, CliError::exit_code)
    }
}

pub fn read_request(reader: &mut impl BufRead, bytes: &mut Vec<u8>) -> io::Result<usize> {
    bytes.clear();
    reader.take(MAX_REQUEST_BYTES as u64 + 2).read_until(b'\n', bytes)
}
