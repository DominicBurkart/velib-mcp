use crate::types::{BikeTypeFilter, Coordinates, DataSource, VelibStation};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicQuery {
    pub center: Coordinates,
    pub radius_meters: u32,
    #[serde(default = "default_limit")]
    pub limit: u16,
}

fn default_limit() -> u16 {
    50
}

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
pub struct StationQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geographic: Option<GeographicQuery>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability: Option<AvailabilityFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub station_codes: Option<Vec<String>>,
    #[serde(default = "default_true")]
    pub include_real_time: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub response_time: DateTime<Utc>,
    pub processing_time_ms: u64,
    pub real_time_source: DataSource,
    pub reference_source: DataSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationListResponse {
    pub stations: Vec<VelibStation>,
    pub total_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationInfo>,
    pub metadata: ResponseMetadata,
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
    pub distance_meters: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyRecommendation {
    pub pickup_station: VelibStation,
    pub dropoff_station: VelibStation,
    pub walk_to_pickup: u32,
    pub walk_from_dropoff: u32,
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
#[serde(tag = "jsonrpc")]
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
#[serde(tag = "jsonrpc")]
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
