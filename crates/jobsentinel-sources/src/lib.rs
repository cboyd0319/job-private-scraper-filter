//! IP Geolocation Module
//!
//! Detects user location from IP address using FreeIPAPI over HTTPS.
//! Used to suggest location after explicit user action.

mod board_urls;
mod job_page;
mod scrapers;

pub use board_urls::{
    is_safe_company_board_id, parse_greenhouse_company_url, parse_lever_company_url,
    CompanyBoardUrl,
};
pub use job_page::{parse_single_job_page, JobPageParseError, ParsedJobPage};
pub use scrapers::{
    limits, GreenhouseCompany, GreenhouseScraper, HnHiringScraper, JobQuery, JobScraper,
    JobsWithGptScraper, LeverCompany, LeverScraper, RateLimiter, RemoteOkScraper, ScraperError,
    UsaJobsScraper, WeWorkRemotelyScraper, LINKEDIN_AUTOMATION_DISABLED_MESSAGE,
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use jobsentinel_network::fetch_external_https_text;

/// Location information detected from IP address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    /// City name (e.g., "Los Angeles")
    pub city: String,
    /// State/region name (e.g., "California")
    pub region: String,
    /// Country name (e.g., "United States")
    pub country: String,
    /// Timezone (e.g., "America/Los_Angeles")
    pub timezone: String,
}

const FREEIPAPI_LOCATION_URL: &str = "https://free.freeipapi.com/api/json";

/// Response from FreeIPAPI.
#[derive(Debug, Deserialize)]
struct FreeIpApiResponse {
    #[serde(rename = "cityName", default)]
    city_name: String,
    #[serde(rename = "regionName", default)]
    region_name: String,
    #[serde(rename = "countryName", default)]
    country_name: String,
    #[serde(rename = "timeZones", default)]
    time_zones: Vec<String>,
    #[serde(default)]
    message: Option<String>,
}

/// Detect location from IP address using FreeIPAPI.
///
/// This is an optional convenience lookup. Frontend callers must keep it behind
/// explicit user action and cache the result to avoid repeated provider calls.
///
/// # Privacy
/// - Calls an external HTTPS geolocation provider only when user requests it
/// - Sends the public IP address visible to that provider
/// - User can override/clear at any time
///
/// # Errors
/// Returns error if:
/// - Network request fails
/// - Provider returns error status or message
/// - Response parsing fails
pub async fn detect_location() -> Result<LocationInfo> {
    let response = fetch_external_https_text(FREEIPAPI_LOCATION_URL, Duration::from_secs(10))
        .await
        .map_err(|_| anyhow::anyhow!("Failed to fetch IP geolocation data"))?;

    if !(200..300).contains(&response.status) {
        bail!(
            "IP geolocation API returned error status: {}",
            response.status
        );
    }

    parse_freeipapi_location_response(&response.body)
}

fn parse_freeipapi_location_response(body: &str) -> Result<LocationInfo> {
    let api_response: FreeIpApiResponse =
        serde_json::from_str(body).context("Failed to parse IP geolocation response")?;

    let city = api_response.city_name.trim().to_string();
    let region = api_response.region_name.trim().to_string();
    let country = api_response.country_name.trim().to_string();
    let timezone = api_response
        .time_zones
        .into_iter()
        .find(|timezone| !timezone.trim().is_empty())
        .unwrap_or_default();

    if city.is_empty() && region.is_empty() {
        if let Some(message) = api_response.message {
            let message = message.trim();
            if !message.is_empty() {
                anyhow::bail!("IP geolocation API error: {}", message);
            }
        }
        anyhow::bail!("IP geolocation returned empty location data");
    }

    Ok(LocationInfo {
        city,
        region,
        country,
        timezone,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Ignore by default (requires network)
    async fn test_detect_location_real_api() {
        // This test requires network access and hits real API.
        // Run with: cargo test -p jobsentinel-sources -- --ignored
        let result = detect_location().await;
        assert!(result.is_ok(), "Failed to detect location: {:?}", result);

        let location = result.unwrap();
        println!("Detected location: {:?}", location);

        // Basic validation - should have at least city or region
        assert!(
            !location.city.is_empty() || !location.region.is_empty(),
            "Location should have city or region"
        );
    }

    #[test]
    fn test_location_info_serialization() {
        let location = LocationInfo {
            city: "San Francisco".to_string(),
            region: "California".to_string(),
            country: "United States".to_string(),
            timezone: "America/Los_Angeles".to_string(),
        };

        let json = serde_json::to_string(&location).unwrap();
        assert!(json.contains("San Francisco"));
        assert!(json.contains("California"));

        let deserialized: LocationInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.city, location.city);
        assert_eq!(deserialized.region, location.region);
    }

    #[test]
    fn test_parse_freeipapi_response_maps_location_fields() {
        let json = r#"{
            "cityName": "Denver",
            "regionName": "Colorado",
            "countryName": "United States",
            "timeZones": ["America/Denver", "America/Chicago"]
        }"#;

        let location = parse_freeipapi_location_response(json).unwrap();

        assert_eq!(location.city, "Denver");
        assert_eq!(location.region, "Colorado");
        assert_eq!(location.country, "United States");
        assert_eq!(location.timezone, "America/Denver");
    }

    #[test]
    fn test_parse_freeipapi_response_rejects_empty_location() {
        let json = r#"{
            "cityName": "",
            "regionName": "",
            "countryName": "United States",
            "timeZones": ["America/Denver"]
        }"#;

        let error = parse_freeipapi_location_response(json).unwrap_err();

        assert!(error
            .to_string()
            .contains("IP geolocation returned empty location data"));
    }
}
