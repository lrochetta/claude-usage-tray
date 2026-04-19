//! Fetch usage data from Anthropic's undocumented OAuth usage endpoint.
//!
//! Auth model: piggy-back on the Claude Code CLI's `.credentials.json` file.
//! We re-read it on every call so that when CC refreshes its token we pick up
//! the new one automatically. We only refresh the token ourselves if CC hasn't
//! done so (token expired + we're called in isolation).
//!
//! ⚠️ The endpoint is undocumented and rate-limited (~5 req then 429 for 30+ min).
//! Callers MUST respect `Config::effective_api_poll_secs()` (minimum 300s).

use crate::config::Config;
use crate::error::{CoreError, Result};
use crate::model::UsageSnapshot;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const REFRESH_URL: &str = "https://console.anthropic.com/v1/oauth/token";
const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const BETA_HEADER: &str = "oauth-2025-04-20";
const USER_AGENT: &str = concat!("claude-usage-tray/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    oauth: OauthCreds,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct OauthCreds {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: i64,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(rename = "subscriptionType", default)]
    subscription_type: Option<String>,
    #[serde(rename = "rateLimitTier", default)]
    rate_limit_tier: Option<String>,
}

/// Convenience: load config, read credentials, fetch.
pub fn fetch_usage() -> Result<UsageSnapshot> {
    let cfg = Config::load_or_default()?;
    fetch_usage_from_credentials(&cfg.credentials_path()?)
}

pub fn fetch_usage_from_credentials(creds_path: &Path) -> Result<UsageSnapshot> {
    let mut creds = read_credentials(creds_path)?;

    // If expired (with 60s safety margin), attempt to refresh.
    let now_ms = jiff::Timestamp::now().as_millisecond();
    if creds.expires_at - 60_000 < now_ms {
        tracing::info!("access token expired or near-expiry, refreshing");
        let refreshed = refresh_token(&creds.refresh_token)?;
        // Write back atomically so CC also picks up the new token.
        write_credentials(creds_path, &refreshed)?;
        creds = refreshed;
    }

    let raw_json = call_usage_endpoint(&creds.access_token)?;
    parse_usage_response(&raw_json)
}

fn read_credentials(path: &Path) -> Result<OauthCreds> {
    if !path.exists() {
        return Err(CoreError::CredentialsNotFound {
            path: path.display().to_string(),
        });
    }
    let text = std::fs::read_to_string(path)?;
    let parsed: CredentialsFile = serde_json::from_str(&text).map_err(|e| {
        CoreError::CredentialsMalformed(format!("parse error in {}: {}", path.display(), e))
    })?;
    Ok(parsed.oauth)
}

fn write_credentials(path: &Path, creds: &OauthCreds) -> Result<()> {
    let wrapper = CredentialsFile {
        oauth: creds.clone(),
    };
    let text = serde_json::to_string_pretty(&wrapper)?;

    // Atomic rename: write to temp, then rename.
    let tmp_path: PathBuf = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, text)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

fn refresh_token(refresh_token: &str) -> Result<OauthCreds> {
    #[derive(Serialize)]
    struct Req<'a> {
        grant_type: &'a str,
        refresh_token: &'a str,
        client_id: &'a str,
    }
    #[derive(Deserialize)]
    struct Resp {
        access_token: String,
        refresh_token: String,
        expires_in: i64,
        #[serde(default)]
        scope: Option<String>,
    }

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .user_agent(USER_AGENT)
        .build();

    let req = Req {
        grant_type: "refresh_token",
        refresh_token,
        client_id: OAUTH_CLIENT_ID,
    };

    let resp = agent
        .post(REFRESH_URL)
        .set("Accept", "application/json")
        .send_json(serde_json::to_value(req)?)?;

    let parsed: Resp = resp
        .into_json()
        .map_err(|e| CoreError::Oauth(format!("refresh response not valid json: {}", e)))?;

    let now_ms = jiff::Timestamp::now().as_millisecond();
    Ok(OauthCreds {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_at: now_ms + parsed.expires_in * 1000,
        scopes: parsed
            .scope
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default(),
        subscription_type: None,
        rate_limit_tier: None,
    })
}

fn call_usage_endpoint(access_token: &str) -> Result<serde_json::Value> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .user_agent(USER_AGENT)
        .build();

    let resp = agent
        .get(USAGE_URL)
        .set("Authorization", &format!("Bearer {}", access_token))
        .set("anthropic-beta", BETA_HEADER)
        .set("Accept", "application/json")
        .call()?;

    let body: serde_json::Value = resp
        .into_json()
        .map_err(|e| CoreError::UnexpectedResponse(format!("invalid json: {}", e)))?;
    Ok(body)
}

/// Parse the response envelope.
///
/// Real response shape (observed 2026-04-19):
/// ```json
/// {
///   "five_hour":  { "utilization": 21.0, "resets_at": "2026-04-19T16:00:00Z" },
///   "seven_day":  { "utilization":  3.0, "resets_at": "2026-04-26T11:00:00Z" },
///   "seven_day_sonnet":  { "utilization": 0.0, "resets_at": null },
///   "seven_day_opus":    null,
///   "seven_day_omelette":{ "utilization": 0.0, "resets_at": null },
///   "extra_usage":       { "is_enabled": false, ... }
/// }
/// ```
///
/// We also accept a few alternate shapes (wrapped `{usage: {...}}`, legacy flat
/// fields) so that tests and unknown future tweaks don't break us silently.
pub fn parse_usage_response(json: &serde_json::Value) -> Result<UsageSnapshot> {
    use serde_json::Value;

    let now_ms = jiff::Timestamp::now().as_millisecond();

    // Unwrap optional { "usage": {...} } envelope.
    let root: &Value = json.get("usage").unwrap_or(json);

    /// Read `root[key].utilization` as f32, or legacy flat `root[key]` / `root[key_usage]`.
    fn nested_pct(root: &Value, key: &str) -> Option<f32> {
        if let Some(obj) = root.get(key) {
            if let Some(u) = obj.get("utilization") {
                if let Some(f) = u.as_f64() {
                    return Some(f as f32);
                }
                if let Some(i) = u.as_i64() {
                    return Some(i as f32);
                }
            }
            // Bare number shape: root[key] = 21.0
            if let Some(f) = obj.as_f64() {
                return Some(f as f32);
            }
            if let Some(i) = obj.as_i64() {
                return Some(i as f32);
            }
        }
        // Legacy flat names
        for flat_key in [format!("{}_usage", key), format!("{}_pct", key)] {
            if let Some(v) = root.get(&flat_key) {
                if let Some(f) = v.as_f64() {
                    return Some(f as f32);
                }
                if let Some(i) = v.as_i64() {
                    return Some(i as f32);
                }
            }
        }
        None
    }

    /// Read `root[key].resets_at` as an ISO-8601 timestamp → epoch ms.
    fn nested_reset(root: &Value, key: &str) -> Option<i64> {
        let candidates = [
            root.get(key).and_then(|o| o.get("resets_at")),
            root.get(format!("{}_reset_at", key)),
            root.get(format!("{}_reset", key)),
        ];
        for cand in candidates.into_iter().flatten() {
            if let Some(s) = cand.as_str() {
                if let Ok(ts) = s.parse::<jiff::Timestamp>() {
                    return Some(ts.as_millisecond());
                }
            }
            if let Some(i) = cand.as_i64() {
                return Some(if i > 1_000_000_000_000 { i } else { i * 1000 });
            }
        }
        None
    }

    fn u32_at(root: &Value, keys: &[&str]) -> Option<u32> {
        for k in keys {
            if let Some(v) = root.get(*k) {
                if let Some(i) = v.as_u64() {
                    return Some(i as u32);
                }
                if let Some(i) = v.as_i64() {
                    return Some(i.max(0) as u32);
                }
            }
        }
        None
    }

    let session_pct = nested_pct(root, "five_hour").unwrap_or(0.0);
    let weekly_all_pct = nested_pct(root, "seven_day").unwrap_or(0.0);
    let weekly_sonnet_pct = nested_pct(root, "seven_day_sonnet");
    // Opus / design tier. Claude UI calls it "Claude Design" but the API field
    // historically has been `seven_day_opus` and more recently `seven_day_omelette`
    // (internal codename). Try both, prefer opus if present.
    let weekly_design_pct = nested_pct(root, "seven_day_opus")
        .or_else(|| nested_pct(root, "seven_day_omelette"))
        .or_else(|| nested_pct(root, "seven_day_design"));

    let session_resets_at_ms = nested_reset(root, "five_hour");
    let weekly_resets_at_ms = nested_reset(root, "seven_day");

    let daily_routines_used = u32_at(
        root,
        &[
            "daily_routines_used",
            "routines_used",
            "daily_included_routine_runs_used",
        ],
    );
    let daily_routines_limit = u32_at(
        root,
        &[
            "daily_routines_limit",
            "routines_limit",
            "daily_included_routine_runs_limit",
        ],
    );

    Ok(UsageSnapshot {
        timestamp_ms: now_ms,
        session_pct: clamp_pct(session_pct),
        session_resets_at_ms,
        weekly_all_pct: clamp_pct(weekly_all_pct),
        weekly_sonnet_pct: weekly_sonnet_pct.map(clamp_pct),
        weekly_design_pct: weekly_design_pct.map(clamp_pct),
        weekly_resets_at_ms,
        daily_routines_used,
        daily_routines_limit,
        raw_payload: Some(json.to_string()),
    })
}

fn clamp_pct(p: f32) -> f32 {
    p.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_real_nested_response() {
        // Shape captured live 2026-04-19 from api.anthropic.com/api/oauth/usage
        let js = serde_json::json!({
            "five_hour": { "utilization": 21.0, "resets_at": "2026-04-19T16:00:00.129978+00:00" },
            "seven_day": { "utilization": 3.0,  "resets_at": "2026-04-26T11:00:00.129992+00:00" },
            "seven_day_sonnet":   { "utilization": 0.0, "resets_at": null },
            "seven_day_opus":     null,
            "seven_day_omelette": { "utilization": 0.0, "resets_at": null },
            "extra_usage": { "is_enabled": false }
        });
        let snap = parse_usage_response(&js).unwrap();
        assert!((snap.session_pct - 21.0).abs() < 0.01);
        assert!((snap.weekly_all_pct - 3.0).abs() < 0.01);
        assert_eq!(snap.weekly_sonnet_pct, Some(0.0));
        // opus is null, falls back to omelette
        assert_eq!(snap.weekly_design_pct, Some(0.0));
        assert!(snap.session_resets_at_ms.is_some());
        assert!(snap.weekly_resets_at_ms.is_some());
    }

    #[test]
    fn parse_wrapped_envelope() {
        let js = serde_json::json!({
            "usage": {
                "five_hour": { "utilization": 78, "resets_at": "2026-04-19T13:10:00Z" },
                "seven_day": { "utilization": 27, "resets_at": "2026-04-23T21:00:00Z" }
            }
        });
        let snap = parse_usage_response(&js).unwrap();
        assert!((snap.session_pct - 78.0).abs() < 0.01);
        assert!((snap.weekly_all_pct - 27.0).abs() < 0.01);
    }

    #[test]
    fn parse_legacy_flat_fields() {
        let js = serde_json::json!({
            "five_hour_usage": 50.5,
            "seven_day_usage": 10.0,
        });
        let snap = parse_usage_response(&js).unwrap();
        assert!((snap.session_pct - 50.5).abs() < 0.01);
    }

    #[test]
    fn missing_fields_default_zero() {
        let js = serde_json::json!({});
        let snap = parse_usage_response(&js).unwrap();
        assert_eq!(snap.session_pct, 0.0);
        assert_eq!(snap.weekly_all_pct, 0.0);
    }
}
