use crate::types::{BikeTypeFilter, Coordinates, VelibStation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AvailabilityFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_bikes: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_docks: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bike_type: Option<BikeTypeFilter>,
    #[serde(default = "default_true")]
    pub exclude_out_of_service: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicBounds {
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
}

impl GeographicBounds {
    #[must_use]
    pub fn contains(&self, coords: &Coordinates) -> bool {
        coords.latitude >= self.south
            && coords.latitude <= self.north
            && coords.longitude >= self.west
            && coords.longitude <= self.east
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationWithDistance {
    #[serde(flatten)]
    pub station: VelibStation,
    pub straight_line_distance_meters: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyRecommendation {
    pub pickup_station: VelibStation,
    pub dropoff_station: VelibStation,
    pub straight_line_to_pickup_meters: u32,
    pub straight_line_from_dropoff_meters: u32,
    pub confidence_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BikeJourney {
    pub pickup_stations: Vec<StationWithDistance>,
    pub dropoff_stations: Vec<StationWithDistance>,
    pub recommendations: Vec<JourneyRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaStatistics {
    pub total_stations: u32,
    pub operational_stations: u32,
    pub total_capacity: u32,
    pub available_bikes: AvailableBikesStats,
    pub available_docks: u32,
    pub occupancy_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableBikesStats {
    pub mechanical: u32,
    pub electric: u32,
    pub total: u32,
}

// MCP Tool Inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindNearbyStationsInput {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default = "default_radius")]
    pub radius_meters: u32,
    #[serde(default = "default_tool_limit")]
    pub limit: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_filter: Option<AvailabilityFilter>,
}

fn default_radius() -> u32 {
    500
}
fn default_tool_limit() -> u16 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStationByCodeInput {
    pub station_code: String,
    #[serde(default = "default_true")]
    pub include_real_time: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStationsByNameInput {
    pub query: String,
    #[serde(default = "default_tool_limit")]
    pub limit: u16,
    #[serde(default = "default_true")]
    pub fuzzy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAreaStatisticsInput {
    pub bounds: GeographicBounds,
    #[serde(default = "default_true")]
    pub include_real_time: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBikeJourneyInput {
    pub origin: Coordinates,
    pub destination: Coordinates,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<JourneyPreferences>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyPreferences {
    #[serde(default)]
    pub bike_type: BikeTypeFilter,
    #[serde(default = "default_max_walk")]
    pub max_walk_distance: u32,
}

fn default_max_walk() -> u32 {
    500
}

// MCP Tool Outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindNearbyStationsOutput {
    pub stations: Vec<StationWithDistance>,
    pub search_metadata: SearchMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetadata {
    pub query_point: Coordinates,
    pub radius_meters: u32,
    pub total_found: u32,
    pub search_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStationByCodeOutput {
    pub station: Option<VelibStation>,
    pub found: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStationsByNameOutput {
    pub stations: Vec<VelibStation>,
    pub search_metadata: TextSearchMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSearchMetadata {
    pub query: String,
    pub total_found: u32,
    pub fuzzy_enabled: bool,
    pub search_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAreaStatisticsOutput {
    pub area_stats: AreaStatistics,
    pub bounds: GeographicBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBikeJourneyOutput {
    pub journey: BikeJourney,
}

// Generic MCP Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(default = "default_jsonrpc")]
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: serde_json::Value,
}

fn default_jsonrpc() -> String {
    "2.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    #[serde(default = "default_jsonrpc")]
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl From<crate::Error> for JsonRpcError {
    fn from(err: crate::Error) -> Self {
        Self {
            code: err.mcp_error_code(),
            message: err.to_string(),
            data: Some(serde_json::json!({
                "error_type": err.error_type()
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Coordinates;
    use serde_json::json;

    // ── existing tests (unchanged) ──────────────────────────────────────────

    #[test]
    fn station_not_found_serializes_null_station() {
        let output = GetStationByCodeOutput {
            station: None,
            found: false,
        };
        let serialized = serde_json::to_string(&output).unwrap();
        assert!(serialized.contains("\"station\":null"));
        assert!(serialized.contains("\"found\":false"));
    }

    #[test]
    fn jsonrpc_response_omits_null_fields() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"ok": true})),
            error: None,
        };
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(!serialized.contains("\"error\""));
        assert!(serialized.contains("\"result\""));
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
    }

    #[test]
    fn jsonrpc_error_from_internal_error() {
        let err = crate::Error::Internal(anyhow::anyhow!("test error"));
        let rpc_err = JsonRpcError::from(err);
        assert_eq!(rpc_err.code, -32603);
        assert!(rpc_err.message.contains("test error"));
    }

    // ── GeographicBounds::contains ──────────────────────────────────────────
    //
    // The bounding-box check is used by get_area_statistics to filter which
    // stations fall inside the query area.  Off-by-one errors or
    // north/south/east/west confusion would silently return wrong counts.

    fn paris_center_bounds() -> GeographicBounds {
        // A tight box around central Paris (roughly 1st–4th arrondissement)
        GeographicBounds {
            north: 48.865,
            south: 48.850,
            east: 2.360,
            west: 2.340,
        }
    }

    #[test]
    fn point_inside_bounds_is_contained() {
        let bounds = paris_center_bounds();
        let inside = Coordinates::new(48.857, 2.350); // Clearly inside
        assert!(bounds.contains(&inside));
    }

    #[test]
    fn point_outside_north_is_not_contained() {
        let bounds = paris_center_bounds();
        let too_far_north = Coordinates::new(48.870, 2.350);
        assert!(!bounds.contains(&too_far_north));
    }

    #[test]
    fn point_outside_south_is_not_contained() {
        let bounds = paris_center_bounds();
        let too_far_south = Coordinates::new(48.840, 2.350);
        assert!(!bounds.contains(&too_far_south));
    }

    #[test]
    fn point_outside_east_is_not_contained() {
        let bounds = paris_center_bounds();
        let too_far_east = Coordinates::new(48.857, 2.370);
        assert!(!bounds.contains(&too_far_east));
    }

    #[test]
    fn point_outside_west_is_not_contained() {
        let bounds = paris_center_bounds();
        let too_far_west = Coordinates::new(48.857, 2.330);
        assert!(!bounds.contains(&too_far_west));
    }

    #[test]
    fn point_on_north_boundary_is_contained() {
        let bounds = paris_center_bounds();
        let on_north = Coordinates::new(48.865, 2.350); // latitude == north
        assert!(bounds.contains(&on_north));
    }

    #[test]
    fn point_on_south_boundary_is_contained() {
        let bounds = paris_center_bounds();
        let on_south = Coordinates::new(48.850, 2.350); // latitude == south
        assert!(bounds.contains(&on_south));
    }

    #[test]
    fn point_on_east_boundary_is_contained() {
        let bounds = paris_center_bounds();
        let on_east = Coordinates::new(48.857, 2.360); // longitude == east
        assert!(bounds.contains(&on_east));
    }

    #[test]
    fn point_on_west_boundary_is_contained() {
        let bounds = paris_center_bounds();
        let on_west = Coordinates::new(48.857, 2.340); // longitude == west
        assert!(bounds.contains(&on_west));
    }

    // ── JsonRpcError::from for key Error variants ───────────────────────────
    //
    // The From impl is the bridge between our domain errors and the wire
    // format an MCP client parses.  We spot-check that the code, message, and
    // error_type data field are all populated correctly for variants beyond
    // the single case covered by the existing test.

    #[test]
    fn jsonrpc_error_from_invalid_coordinates() {
        let err = crate::Error::InvalidCoordinates {
            latitude: 0.0,
            longitude: 0.0,
        };
        let rpc = JsonRpcError::from(err);
        assert_eq!(rpc.code, -32602);
        let data = rpc.data.unwrap();
        assert_eq!(data["error_type"], "invalid_coordinates");
    }

    #[test]
    fn jsonrpc_error_from_station_not_found() {
        let err = crate::Error::StationNotFound {
            station_code: "42".to_string(),
        };
        let rpc = JsonRpcError::from(err);
        assert_eq!(rpc.code, -32600);
        assert!(rpc.message.contains("42"));
        let data = rpc.data.unwrap();
        assert_eq!(data["error_type"], "station_not_found");
    }

    #[test]
    fn jsonrpc_error_from_search_radius_too_large() {
        let err = crate::Error::SearchRadiusTooLarge {
            radius: 9999,
            max: 5000,
        };
        let rpc = JsonRpcError::from(err);
        assert_eq!(rpc.code, -32602);
        let data = rpc.data.unwrap();
        assert_eq!(data["error_type"], "search_radius_too_large");
    }

    #[test]
    fn jsonrpc_error_always_includes_error_type_in_data() {
        // Whatever the error variant, the data object must carry an
        // "error_type" key so clients can reliably branch on it.
        let errors: Vec<crate::Error> = vec![
            crate::Error::Validation("x".into()),
            crate::Error::Cache("x".into()),
            crate::Error::McpProtocol("x".into()),
            crate::Error::OutsideServiceArea { distance_km: 100.0 },
            crate::Error::ResultLimitExceeded { limit: 200, max: 100 },
        ];
        for err in errors {
            let rpc = JsonRpcError::from(err);
            let data = rpc.data.expect("data field must be present");
            assert!(
                data["error_type"].is_string(),
                "error_type must be a string in data"
            );
        }
    }

    // ── JsonRpcResponse – error path ────────────────────────────────────────

    #[test]
    fn jsonrpc_response_with_error_omits_result() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(99),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "bad request".to_string(),
                data: None,
            }),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(!serialized.contains("\"result\""));
        assert!(serialized.contains("\"error\""));
        assert!(serialized.contains("-32600"));
    }

    // ── default serde values ────────────────────────────────────────────────

    #[test]
    fn find_nearby_stations_input_defaults() {
        // Deserialise with only the required fields; verify serde defaults.
        let input: FindNearbyStationsInput =
            serde_json::from_str(r#"{"latitude":48.856,"longitude":2.352}"#).unwrap();
        assert_eq!(input.radius_meters, 500);
        assert_eq!(input.limit, 10);
        assert!(input.availability_filter.is_none());
    }

    #[test]
    fn journey_preferences_defaults_to_any_bike_type() {
        use crate::types::BikeTypeFilter;
        let prefs: JourneyPreferences =
            serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(prefs.bike_type, BikeTypeFilter::AnyType);
        assert_eq!(prefs.max_walk_distance, 500);
    }
}
