use crate::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Output format for documentation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum DocumentationFormat {
    #[serde(rename = "json")]
    #[default]
    Json,
    #[serde(rename = "openapi")]
    OpenApi,
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "csv")]
    Csv,
}

/// Configuration for documentation generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationConfig {
    pub format: DocumentationFormat,
    pub include_examples: bool,
    pub token_efficient: bool,
    pub include_metadata: bool,
    pub version: String,
}

impl Default for DocumentationConfig {
    fn default() -> Self {
        Self {
            format: DocumentationFormat::Json,
            include_examples: true,
            token_efficient: false,
            include_metadata: true,
            version: "1.0.0".to_string(),
        }
    }
}

/// Schema documentation for MCP tools and resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDocumentation {
    pub version: String,
    pub last_updated: DateTime<Utc>,
    pub server_info: ServerInfo,
    pub tools: Vec<ToolDocumentation>,
    pub resources: Vec<ResourceDocumentation>,
    pub types: HashMap<String, TypeDocumentation>,
    pub error_codes: Vec<ErrorCodeDocumentation>,
    pub rate_limits: RateLimitDocumentation,
    pub usage_guidelines: UsageGuidelines,
}

/// Server information for LLM context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub service_area: ServiceArea,
    pub data_sources: Vec<DataSourceInfo>,
    pub capabilities: Vec<String>,
}

/// Geographic service area information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceArea {
    pub description: String,
    pub center: Coordinates,
    pub radius_km: f64,
    pub bounds: GeographicBounds,
}

/// Geographic bounds for service area
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicBounds {
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
}

/// Data source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceInfo {
    pub name: String,
    pub description: String,
    pub update_frequency: String,
    pub cache_ttl_seconds: u64,
    pub reliability: String,
}

/// Tool documentation with LLM-friendly descriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDocumentation {
    pub name: String,
    pub description: String,
    pub purpose: String,
    pub use_cases: Vec<String>,
    pub input_schema: Value,
    pub output_schema: Value,
    pub examples: Vec<ToolExample>,
    pub constraints: Vec<String>,
    pub performance_notes: Vec<String>,
}

/// Resource documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDocumentation {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub content_type: String,
    pub data_freshness: DataFreshness,
    pub cache_info: CacheInfo,
    pub schema: Value,
    pub sample_response: Option<Value>,
}

/// Cache information for resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub ttl_seconds: u64,
    pub update_frequency: String,
    pub last_updated: Option<DateTime<Utc>>,
}

/// Type documentation for complex types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDocumentation {
    pub name: String,
    pub description: String,
    pub schema: Value,
    pub examples: Vec<Value>,
    pub validation_rules: Vec<String>,
}

/// Tool usage example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub title: String,
    pub description: String,
    pub input: Value,
    pub output: Value,
    pub notes: Option<String>,
}

/// Error code documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCodeDocumentation {
    pub code: i32,
    pub name: String,
    pub description: String,
    pub when_occurs: String,
    pub how_to_fix: String,
    pub example: Value,
}

/// Rate limiting information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitDocumentation {
    pub tools_per_minute: u32,
    pub resources_per_minute: u32,
    pub burst_limit: u32,
    pub headers: Vec<String>,
}

/// Usage guidelines for LLMs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageGuidelines {
    pub best_practices: Vec<String>,
    pub common_patterns: Vec<String>,
    pub performance_tips: Vec<String>,
    pub troubleshooting: Vec<String>,
}

/// Main documentation generator
pub struct DocumentationGenerator {
    config: DocumentationConfig,
}

impl DocumentationGenerator {
    pub fn new(config: DocumentationConfig) -> Self {
        Self { config }
    }

    pub fn with_format(format: DocumentationFormat) -> Self {
        Self::new(DocumentationConfig {
            format,
            ..Default::default()
        })
    }

    /// Generate complete MCP documentation
    pub fn generate_documentation(&self) -> McpDocumentation {
        McpDocumentation {
            version: self.config.version.clone(),
            last_updated: Utc::now(),
            server_info: self.generate_server_info(),
            tools: self.generate_tools_documentation(),
            resources: self.generate_resources_documentation(),
            types: self.generate_types_documentation(),
            error_codes: self.generate_error_codes(),
            rate_limits: self.generate_rate_limits(),
            usage_guidelines: self.generate_usage_guidelines(),
        }
    }

    /// Generate server information
    fn generate_server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "velib-mcp".to_string(),
            description: "Paris Velib bike-sharing data MCP server providing real-time station information and journey planning tools".to_string(),
            version: "1.0.0".to_string(),
            service_area: ServiceArea {
                description: "Paris metropolitan area within 50km of Paris City Hall".to_string(),
                center: Coordinates::new(48.8565, 2.3514), // Paris City Hall
                radius_km: 50.0,
                bounds: GeographicBounds {
                    north: 49.0,
                    south: 48.7,
                    east: 2.6,
                    west: 2.0,
                },
            },
            data_sources: vec![
                DataSourceInfo {
                    name: "Paris Open Data - Real-time".to_string(),
                    description: "Live bike and dock availability updated every 60 seconds".to_string(),
                    update_frequency: "60 seconds".to_string(),
                    cache_ttl_seconds: 90,
                    reliability: "99.5% uptime".to_string(),
                },
                DataSourceInfo {
                    name: "Paris Open Data - Reference".to_string(),
                    description: "Station locations and metadata updated daily".to_string(),
                    update_frequency: "Daily at 06:00 UTC".to_string(),
                    cache_ttl_seconds: 3600,
                    reliability: "99.9% uptime".to_string(),
                },
            ],
            capabilities: vec![
                "Real-time bike availability".to_string(),
                "Station location search".to_string(),
                "Journey planning".to_string(),
                "Area statistics".to_string(),
                "Geographic filtering".to_string(),
            ],
        }
    }

    /// Generate tools documentation
    fn generate_tools_documentation(&self) -> Vec<ToolDocumentation> {
        vec![
            self.generate_find_nearby_stations_doc(),
            self.generate_get_station_by_code_doc(),
            self.generate_search_stations_by_name_doc(),
            self.generate_get_area_statistics_doc(),
            self.generate_plan_bike_journey_doc(),
        ]
    }

    /// Generate find_nearby_stations documentation
    fn generate_find_nearby_stations_doc(&self) -> ToolDocumentation {
        ToolDocumentation {
            name: "find_nearby_stations".to_string(),
            description: "Find Velib stations within walking distance of a location".to_string(),
            purpose: "Locate bike stations near a specific point for pickup or dropoff".to_string(),
            use_cases: vec![
                "Find stations near current location".to_string(),
                "Check bike availability before travel".to_string(),
                "Find stations with specific bike types".to_string(),
                "Locate stations with available docks".to_string(),
            ],
            input_schema: json!({
                "type": "object",
                "properties": {
                    "latitude": {
                        "type": "number",
                        "minimum": 48.7,
                        "maximum": 49.0,
                        "description": "Latitude in decimal degrees (Paris metro area)"
                    },
                    "longitude": {
                        "type": "number",
                        "minimum": 2.0,
                        "maximum": 2.6,
                        "description": "Longitude in decimal degrees (Paris metro area)"
                    },
                    "radius_meters": {
                        "type": "integer",
                        "minimum": 100,
                        "maximum": 5000,
                        "default": 500,
                        "description": "Search radius in meters (walking distance)"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "default": 10,
                        "description": "Maximum number of stations to return"
                    },
                    "availability_filter": {
                        "type": "object",
                        "properties": {
                            "bike_type": {
                                "type": "string",
                                "enum": ["mechanical", "electric", "any"],
                                "default": "any",
                                "description": "Required bike type availability"
                            }
                        }
                    }
                },
                "required": ["latitude", "longitude"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "search_metadata": {
                        "type": "object",
                        "properties": {
                            "query_point": {"$ref": "#/definitions/Coordinates"},
                            "radius_meters": {"type": "integer"},
                            "total_found": {"type": "integer"},
                            "search_time_ms": {"type": "integer"}
                        }
                    },
                    "stations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "station": {"$ref": "#/definitions/VelibStation"},
                                "distance_meters": {"type": "integer"}
                            }
                        }
                    }
                }
            }),
            examples: if self.config.include_examples {
                vec![ToolExample {
                    title: "Find nearby stations in central Paris".to_string(),
                    description: "Search for bike stations within 500m of the Louvre".to_string(),
                    input: json!({
                        "latitude": 48.8606,
                        "longitude": 2.3376,
                        "radius_meters": 500,
                        "limit": 5
                    }),
                    output: json!({
                        "search_metadata": {
                            "query_point": {"latitude": 48.8606, "longitude": 2.3376},
                            "radius_meters": 500,
                            "total_found": 3,
                            "search_time_ms": 45
                        },
                        "stations": [
                            {
                                "station": {
                                    "reference": {
                                        "station_code": "1001",
                                        "name": "Louvre - Rivoli",
                                        "coordinates": {"latitude": 48.8608, "longitude": 2.3372},
                                        "capacity": 25
                                    },
                                    "real_time": {
                                        "bikes": {"mechanical": 8, "electric": 4},
                                        "available_docks": 13,
                                        "status": "Open"
                                    }
                                },
                                "distance_meters": 35
                            }
                        ]
                    }),
                    notes: Some("Response truncated for brevity".to_string()),
                }]
            } else {
                vec![]
            },
            constraints: vec![
                "Search limited to Paris metropolitan area (48.7-49.0°N, 2.0-2.6°E)".to_string(),
                "Maximum radius: 5000 meters".to_string(),
                "Maximum results: 100 stations".to_string(),
                "Service area: within 50km of Paris City Hall".to_string(),
            ],
            performance_notes: vec![
                "Typical response time: 50-200ms".to_string(),
                "Results sorted by distance (closest first)".to_string(),
                "Live data refreshed every 60 seconds".to_string(),
            ],
        }
    }

    /// Generate get_station_by_code documentation
    fn generate_get_station_by_code_doc(&self) -> ToolDocumentation {
        ToolDocumentation {
            name: "get_station_by_code".to_string(),
            description: "Get detailed information about a specific Velib station".to_string(),
            purpose: "Retrieve complete station details including real-time availability"
                .to_string(),
            use_cases: vec![
                "Check status of a known station".to_string(),
                "Get detailed station information".to_string(),
                "Verify station exists and is operational".to_string(),
            ],
            input_schema: json!({
                "type": "object",
                "properties": {
                    "station_code": {
                        "type": "string",
                        "pattern": "^[0-9]+$",
                        "description": "Unique station identifier (numeric string)"
                    }
                },
                "required": ["station_code"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "found": {
                        "type": "boolean",
                        "description": "Whether the station was found"
                    },
                    "station": {
                        "anyOf": [
                            {"$ref": "#/definitions/VelibStation"},
                            {"type": "null"}
                        ]
                    }
                }
            }),
            examples: if self.config.include_examples {
                vec![ToolExample {
                    title: "Get station by code".to_string(),
                    description: "Retrieve information for station 32017".to_string(),
                    input: json!({"station_code": "32017"}),
                    output: json!({
                        "found": true,
                        "station": {
                            "reference": {
                                "station_code": "32017",
                                "name": "Rouget de L'isle - Watteau",
                                "coordinates": {"latitude": 48.936268, "longitude": 2.358866},
                                "capacity": 22
                            },
                            "real_time": {
                                "bikes": {"mechanical": 8, "electric": 4},
                                "available_docks": 10,
                                "status": "Open"
                            }
                        }
                    }),
                    notes: None,
                }]
            } else {
                vec![]
            },
            constraints: vec![
                "Station code must be numeric".to_string(),
                "Returns null if station not found".to_string(),
            ],
            performance_notes: vec![
                "Typical response time: 30-100ms".to_string(),
                "Includes latest real-time data".to_string(),
            ],
        }
    }

    /// Generate search_stations_by_name documentation
    fn generate_search_stations_by_name_doc(&self) -> ToolDocumentation {
        ToolDocumentation {
            name: "search_stations_by_name".to_string(),
            description: "Search for stations by name with fuzzy matching".to_string(),
            purpose: "Find stations when you know part of the name but not the exact code"
                .to_string(),
            use_cases: vec![
                "Find stations by landmark name".to_string(),
                "Search for stations near a street or area".to_string(),
                "Discover stations by partial name match".to_string(),
            ],
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "minLength": 2,
                        "description": "Search term (minimum 2 characters)"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 50,
                        "default": 10,
                        "description": "Maximum number of results"
                    },
                    "fuzzy": {
                        "type": "boolean",
                        "default": true,
                        "description": "Enable fuzzy matching (substring search)"
                    }
                },
                "required": ["query"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "search_metadata": {
                        "type": "object",
                        "properties": {
                            "query": {"type": "string"},
                            "total_found": {"type": "integer"},
                            "fuzzy_enabled": {"type": "boolean"},
                            "search_time_ms": {"type": "integer"}
                        }
                    },
                    "stations": {
                        "type": "array",
                        "items": {"$ref": "#/definitions/VelibStation"}
                    }
                }
            }),
            examples: if self.config.include_examples {
                vec![ToolExample {
                    title: "Search for stations by name".to_string(),
                    description: "Find stations with 'Louvre' in the name".to_string(),
                    input: json!({
                        "query": "Louvre",
                        "limit": 3,
                        "fuzzy": true
                    }),
                    output: json!({
                        "search_metadata": {
                            "query": "Louvre",
                            "total_found": 2,
                            "fuzzy_enabled": true,
                            "search_time_ms": 25
                        },
                        "stations": [
                            {
                                "reference": {
                                    "station_code": "1001",
                                    "name": "Louvre - Rivoli",
                                    "coordinates": {"latitude": 48.8608, "longitude": 2.3372},
                                    "capacity": 25
                                }
                            }
                        ]
                    }),
                    notes: Some("Results sorted alphabetically".to_string()),
                }]
            } else {
                vec![]
            },
            constraints: vec![
                "Minimum query length: 2 characters".to_string(),
                "Maximum results: 50 stations".to_string(),
                "Case-insensitive search".to_string(),
            ],
            performance_notes: vec![
                "Fuzzy search may be slower than exact match".to_string(),
                "Results sorted alphabetically by name".to_string(),
            ],
        }
    }

    /// Generate get_area_statistics documentation
    fn generate_get_area_statistics_doc(&self) -> ToolDocumentation {
        ToolDocumentation {
            name: "get_area_statistics".to_string(),
            description: "Get aggregated statistics for a geographic area".to_string(),
            purpose: "Analyze bike availability and station density in a specific region"
                .to_string(),
            use_cases: vec![
                "Analyze bike availability in a neighborhood".to_string(),
                "Get occupancy statistics for an area".to_string(),
                "Compare different areas of Paris".to_string(),
                "Monitor system capacity and usage".to_string(),
            ],
            input_schema: json!({
                "type": "object",
                "properties": {
                    "bounds": {
                        "type": "object",
                        "properties": {
                            "north": {"type": "number", "description": "Northern boundary (latitude)"},
                            "south": {"type": "number", "description": "Southern boundary (latitude)"},
                            "east": {"type": "number", "description": "Eastern boundary (longitude)"},
                            "west": {"type": "number", "description": "Western boundary (longitude)"}
                        },
                        "required": ["north", "south", "east", "west"]
                    }
                },
                "required": ["bounds"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "area_stats": {
                        "type": "object",
                        "properties": {
                            "total_stations": {"type": "integer"},
                            "operational_stations": {"type": "integer"},
                            "total_capacity": {"type": "integer"},
                            "available_bikes": {
                                "type": "object",
                                "properties": {
                                    "mechanical": {"type": "integer"},
                                    "electric": {"type": "integer"},
                                    "total": {"type": "integer"}
                                }
                            },
                            "available_docks": {"type": "integer"},
                            "occupancy_rate": {
                                "type": "number",
                                "minimum": 0,
                                "maximum": 1,
                                "description": "Occupancy rate as decimal (0.0-1.0)"
                            }
                        }
                    },
                    "bounds": {"$ref": "#/definitions/GeographicBounds"}
                }
            }),
            examples: if self.config.include_examples {
                vec![ToolExample {
                    title: "Get statistics for central Paris".to_string(),
                    description: "Analyze bike availability in the 1st arrondissement".to_string(),
                    input: json!({
                        "bounds": {
                            "north": 48.8700,
                            "south": 48.8550,
                            "east": 2.3450,
                            "west": 2.3250
                        }
                    }),
                    output: json!({
                        "area_stats": {
                            "total_stations": 24,
                            "operational_stations": 22,
                            "total_capacity": 480,
                            "available_bikes": {
                                "mechanical": 156,
                                "electric": 89,
                                "total": 245
                            },
                            "available_docks": 235,
                            "occupancy_rate": 0.51
                        }
                    }),
                    notes: Some("Statistics calculated from live data".to_string()),
                }]
            } else {
                vec![]
            },
            constraints: vec![
                "Bounds must be within Paris service area".to_string(),
                "Statistics calculated from operational stations only".to_string(),
            ],
            performance_notes: vec![
                "Response time increases with area size".to_string(),
                "Statistics reflect real-time data".to_string(),
            ],
        }
    }

    /// Generate plan_bike_journey documentation
    fn generate_plan_bike_journey_doc(&self) -> ToolDocumentation {
        ToolDocumentation {
            name: "plan_bike_journey".to_string(),
            description: "Plan a bike journey with optimal pickup and dropoff stations".to_string(),
            purpose: "Find the best stations for starting and ending a bike trip".to_string(),
            use_cases: vec![
                "Plan a trip from point A to point B".to_string(),
                "Find optimal pickup station near origin".to_string(),
                "Find suitable dropoff station near destination".to_string(),
                "Get journey recommendations with confidence scores".to_string(),
            ],
            input_schema: json!({
                "type": "object",
                "properties": {
                    "origin": {
                        "type": "object",
                        "properties": {
                            "latitude": {"type": "number"},
                            "longitude": {"type": "number"}
                        },
                        "required": ["latitude", "longitude"]
                    },
                    "destination": {
                        "type": "object",
                        "properties": {
                            "latitude": {"type": "number"},
                            "longitude": {"type": "number"}
                        },
                        "required": ["latitude", "longitude"]
                    },
                    "preferences": {
                        "type": "object",
                        "properties": {
                            "bike_type": {
                                "type": "string",
                                "enum": ["mechanical", "electric", "any"],
                                "default": "any"
                            },
                            "max_walk_distance": {
                                "type": "integer",
                                "default": 500,
                                "description": "Maximum walking distance in meters"
                            }
                        }
                    }
                },
                "required": ["origin", "destination"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "journey": {
                        "type": "object",
                        "properties": {
                            "pickup_stations": {
                                "type": "array",
                                "items": {"$ref": "#/definitions/StationWithDistance"}
                            },
                            "dropoff_stations": {
                                "type": "array",
                                "items": {"$ref": "#/definitions/StationWithDistance"}
                            },
                            "recommendations": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "pickup_station": {"$ref": "#/definitions/VelibStation"},
                                        "dropoff_station": {"$ref": "#/definitions/VelibStation"},
                                        "walk_to_pickup": {"type": "integer", "description": "Walking distance to pickup (meters)"},
                                        "walk_from_dropoff": {"type": "integer", "description": "Walking distance from dropoff (meters)"},
                                        "confidence_score": {
                                            "type": "number",
                                            "minimum": 0,
                                            "maximum": 1,
                                            "description": "Recommendation confidence (0.0-1.0)"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }),
            examples: if self.config.include_examples {
                vec![ToolExample {
                    title: "Plan journey from Louvre to Eiffel Tower".to_string(),
                    description: "Find optimal stations for a tourist trip".to_string(),
                    input: json!({
                        "origin": {"latitude": 48.8606, "longitude": 2.3376},
                        "destination": {"latitude": 48.8584, "longitude": 2.2945},
                        "preferences": {
                            "bike_type": "any",
                            "max_walk_distance": 400
                        }
                    }),
                    output: json!({
                        "journey": {
                            "pickup_stations": [
                                {
                                    "station": {
                                        "reference": {
                                            "station_code": "1001",
                                            "name": "Louvre - Rivoli",
                                            "coordinates": {"latitude": 48.8608, "longitude": 2.3372}
                                        }
                                    },
                                    "distance_meters": 35
                                }
                            ],
                            "dropoff_stations": [
                                {
                                    "station": {
                                        "reference": {
                                            "station_code": "7001",
                                            "name": "Tour Eiffel",
                                            "coordinates": {"latitude": 48.8582, "longitude": 2.2950}
                                        }
                                    },
                                    "distance_meters": 42
                                }
                            ],
                            "recommendations": [
                                {
                                    "pickup_station": {"reference": {"station_code": "1001"}},
                                    "dropoff_station": {"reference": {"station_code": "7001"}},
                                    "walk_to_pickup": 35,
                                    "walk_from_dropoff": 42,
                                    "confidence_score": 0.95
                                }
                            ]
                        }
                    }),
                    notes: Some(
                        "Confidence based on walking distance and availability".to_string(),
                    ),
                }]
            } else {
                vec![]
            },
            constraints: vec![
                "Both origin and destination must be within service area".to_string(),
                "Maximum walking distance: 2000 meters".to_string(),
                "Recommendations sorted by confidence score".to_string(),
            ],
            performance_notes: vec![
                "Considers real-time bike and dock availability".to_string(),
                "Confidence score factors in walking distance".to_string(),
            ],
        }
    }

    /// Generate resources documentation
    fn generate_resources_documentation(&self) -> Vec<ResourceDocumentation> {
        vec![
            ResourceDocumentation {
                uri: "velib://stations/reference".to_string(),
                name: "Velib Station Reference Data".to_string(),
                description: "Complete catalog of Velib stations with static metadata including locations, names, and capacity".to_string(),
                content_type: "application/json".to_string(),
                data_freshness: DataFreshness::Recent,
                cache_info: CacheInfo {
                    ttl_seconds: 3600,
                    update_frequency: "Daily at 06:00 UTC".to_string(),
                    last_updated: Some(Utc::now()),
                },
                schema: json!({
                    "type": "object",
                    "properties": {
                        "stations": {
                            "type": "array",
                            "items": {"$ref": "#/definitions/StationReference"}
                        },
                        "metadata": {
                            "type": "object",
                            "properties": {
                                "total_stations": {"type": "integer"},
                                "last_updated": {"type": "string", "format": "date-time"}
                            }
                        }
                    }
                }),
                sample_response: if self.config.include_examples {
                    Some(json!({
                        "stations": [
                            {
                                "station_code": "32017",
                                "name": "Rouget de L'isle - Watteau",
                                "coordinates": {"latitude": 48.936268, "longitude": 2.358866},
                                "capacity": 22
                            }
                        ],
                        "metadata": {
                            "total_stations": 1447,
                            "last_updated": "2025-01-11T06:00:00Z"
                        }
                    }))
                } else {
                    None
                },
            },
            ResourceDocumentation {
                uri: "velib://stations/realtime".to_string(),
                name: "Velib Real-time Availability".to_string(),
                description: "Current bike and dock availability for all stations updated every 60 seconds".to_string(),
                content_type: "application/json".to_string(),
                data_freshness: DataFreshness::Fresh,
                cache_info: CacheInfo {
                    ttl_seconds: 90,
                    update_frequency: "Every 60 seconds".to_string(),
                    last_updated: Some(Utc::now()),
                },
                schema: json!({
                    "type": "object",
                    "properties": {
                        "stations": {
                            "type": "array",
                            "items": {"$ref": "#/definitions/RealTimeStatus"}
                        },
                        "metadata": {
                            "type": "object",
                            "properties": {
                                "data_freshness": {"type": "string", "enum": ["Fresh", "Recent", "Stale", "VeryStale"]},
                                "response_time": {"type": "string", "format": "date-time"}
                            }
                        }
                    }
                }),
                sample_response: None,
            },
            ResourceDocumentation {
                uri: "velib://stations/complete".to_string(),
                name: "Velib Complete Station Data".to_string(),
                description: "Combined reference and real-time data for all stations in a single response".to_string(),
                content_type: "application/json".to_string(),
                data_freshness: DataFreshness::Fresh,
                cache_info: CacheInfo {
                    ttl_seconds: 90,
                    update_frequency: "Every 60 seconds".to_string(),
                    last_updated: Some(Utc::now()),
                },
                schema: json!({
                    "type": "object",
                    "properties": {
                        "stations": {
                            "type": "array",
                            "items": {"$ref": "#/definitions/VelibStation"}
                        },
                        "metadata": {
                            "type": "object",
                            "properties": {
                                "data_freshness": {"type": "string"},
                                "response_time": {"type": "string", "format": "date-time"}
                            }
                        }
                    }
                }),
                sample_response: None,
            },
            ResourceDocumentation {
                uri: "velib://health".to_string(),
                name: "Service Health Status".to_string(),
                description: "System health and data source status information for monitoring".to_string(),
                content_type: "application/json".to_string(),
                data_freshness: DataFreshness::Fresh,
                cache_info: CacheInfo {
                    ttl_seconds: 30,
                    update_frequency: "Real-time".to_string(),
                    last_updated: Some(Utc::now()),
                },
                schema: json!({
                    "type": "object",
                    "properties": {
                        "status": {"type": "string", "enum": ["healthy", "degraded", "unhealthy"]},
                        "version": {"type": "string"},
                        "uptime_seconds": {"type": "integer"},
                        "data_sources": {
                            "type": "object",
                            "properties": {
                                "real_time": {"$ref": "#/definitions/DataSourceStatus"},
                                "reference": {"$ref": "#/definitions/DataSourceStatus"}
                            }
                        },
                        "cache_stats": {
                            "type": "object",
                            "properties": {
                                "hit_rate": {"type": "number", "minimum": 0, "maximum": 1},
                                "entries": {"type": "integer"}
                            }
                        }
                    }
                }),
                sample_response: None,
            },
        ]
    }

    /// Generate types documentation
    fn generate_types_documentation(&self) -> HashMap<String, TypeDocumentation> {
        let mut types = HashMap::new();

        types.insert("Coordinates".to_string(), TypeDocumentation {
            name: "Coordinates".to_string(),
            description: "Geographic coordinates in decimal degrees".to_string(),
            schema: json!({
                "type": "object",
                "properties": {
                    "latitude": {"type": "number", "description": "Latitude in decimal degrees"},
                    "longitude": {"type": "number", "description": "Longitude in decimal degrees"}
                },
                "required": ["latitude", "longitude"]
            }),
            examples: vec![
                json!({"latitude": 48.8566, "longitude": 2.3522}),
                json!({"latitude": 48.8584, "longitude": 2.2945}),
            ],
            validation_rules: vec![
                "Latitude must be between -90 and 90".to_string(),
                "Longitude must be between -180 and 180".to_string(),
                "For this service: latitude 48.7-49.0, longitude 2.0-2.6".to_string(),
            ],
        });

        types.insert("BikeAvailability".to_string(), TypeDocumentation {
            name: "BikeAvailability".to_string(),
            description: "Available bikes by type at a station".to_string(),
            schema: json!({
                "type": "object",
                "properties": {
                    "mechanical": {"type": "integer", "minimum": 0, "description": "Number of mechanical bikes"},
                    "electric": {"type": "integer", "minimum": 0, "description": "Number of electric bikes"}
                },
                "required": ["mechanical", "electric"]
            }),
            examples: vec![
                json!({"mechanical": 5, "electric": 3}),
                json!({"mechanical": 0, "electric": 7}),
            ],
            validation_rules: vec![
                "Both values must be non-negative integers".to_string(),
                "Total bikes cannot exceed station capacity".to_string(),
            ],
        });

        types.insert(
            "StationStatus".to_string(),
            TypeDocumentation {
                name: "StationStatus".to_string(),
                description: "Operational status of a station".to_string(),
                schema: json!({
                    "type": "string",
                    "enum": ["Open", "Closed", "Maintenance"],
                    "description": "Current operational status"
                }),
                examples: vec![json!("Open"), json!("Closed"), json!("Maintenance")],
                validation_rules: vec![
                    "Open: Station is operational for pickup and dropoff".to_string(),
                    "Closed: Station is temporarily closed".to_string(),
                    "Maintenance: Station is under maintenance".to_string(),
                ],
            },
        );

        types.insert(
            "DataFreshness".to_string(),
            TypeDocumentation {
                name: "DataFreshness".to_string(),
                description: "Indicates how recent the data is".to_string(),
                schema: json!({
                    "type": "string",
                    "enum": ["Fresh", "Recent", "Stale", "VeryStale"],
                    "description": "Data freshness indicator"
                }),
                examples: vec![json!("Fresh"), json!("Recent")],
                validation_rules: vec![
                    "Fresh: Data is less than 5 minutes old".to_string(),
                    "Recent: Data is 5-15 minutes old".to_string(),
                    "Stale: Data is 15-60 minutes old".to_string(),
                    "VeryStale: Data is more than 60 minutes old".to_string(),
                ],
            },
        );

        types.insert(
            "BikeTypeFilter".to_string(),
            TypeDocumentation {
                name: "BikeTypeFilter".to_string(),
                description: "Filter for required bike types".to_string(),
                schema: json!({
                    "type": "string",
                    "enum": ["mechanical", "electric", "any"],
                    "description": "Required bike type availability"
                }),
                examples: vec![json!("any"), json!("electric"), json!("mechanical")],
                validation_rules: vec![
                    "mechanical: Station must have at least one mechanical bike".to_string(),
                    "electric: Station must have at least one electric bike".to_string(),
                    "any: Station can have any type of bike".to_string(),
                ],
            },
        );

        types
    }

    /// Generate error codes documentation
    fn generate_error_codes(&self) -> Vec<ErrorCodeDocumentation> {
        vec![
            ErrorCodeDocumentation {
                code: -32001,
                name: "STATION_NOT_FOUND".to_string(),
                description: "The requested station does not exist".to_string(),
                when_occurs: "When get_station_by_code is called with an invalid station code".to_string(),
                how_to_fix: "Verify the station code exists using search_stations_by_name or find_nearby_stations".to_string(),
                example: json!({
                    "error": {
                        "code": -32001,
                        "message": "Station not found",
                        "data": {"station_code": "99999"}
                    }
                }),
            },
            ErrorCodeDocumentation {
                code: -32002,
                name: "INVALID_COORDINATES".to_string(),
                description: "Provided coordinates are invalid or outside service area".to_string(),
                when_occurs: "When latitude/longitude are outside Paris metropolitan area bounds".to_string(),
                how_to_fix: "Ensure coordinates are within Paris area: lat 48.7-49.0, lon 2.0-2.6".to_string(),
                example: json!({
                    "error": {
                        "code": -32002,
                        "message": "Invalid coordinates",
                        "data": {"latitude": 40.7128, "longitude": -74.0060}
                    }
                }),
            },
            ErrorCodeDocumentation {
                code: -32003,
                name: "OUTSIDE_SERVICE_AREA".to_string(),
                description: "Location is outside the 50km service area from Paris City Hall".to_string(),
                when_occurs: "When coordinates are valid but outside the service boundary".to_string(),
                how_to_fix: "Use coordinates within 50km of Paris City Hall (48.8565°N, 2.3514°E)".to_string(),
                example: json!({
                    "error": {
                        "code": -32003,
                        "message": "Outside service area",
                        "data": {"distance_km": 75.3}
                    }
                }),
            },
            ErrorCodeDocumentation {
                code: -32004,
                name: "SEARCH_RADIUS_TOO_LARGE".to_string(),
                description: "Search radius exceeds the maximum allowed value".to_string(),
                when_occurs: "When radius_meters parameter is greater than 5000".to_string(),
                how_to_fix: "Use a radius of 5000 meters or less".to_string(),
                example: json!({
                    "error": {
                        "code": -32004,
                        "message": "Search radius too large",
                        "data": {"radius": 10000, "max": 5000}
                    }
                }),
            },
            ErrorCodeDocumentation {
                code: -32005,
                name: "RESULT_LIMIT_EXCEEDED".to_string(),
                description: "Requested result limit exceeds maximum allowed".to_string(),
                when_occurs: "When limit parameter exceeds tool-specific maximum".to_string(),
                how_to_fix: "Reduce limit parameter (typically max 100 for tools, 50 for searches)".to_string(),
                example: json!({
                    "error": {
                        "code": -32005,
                        "message": "Result limit exceeded",
                        "data": {"limit": 200, "max": 100}
                    }
                }),
            },
        ]
    }

    /// Generate rate limits documentation
    fn generate_rate_limits(&self) -> RateLimitDocumentation {
        RateLimitDocumentation {
            tools_per_minute: 100,
            resources_per_minute: 60,
            burst_limit: 10,
            headers: vec![
                "X-RateLimit-Limit".to_string(),
                "X-RateLimit-Remaining".to_string(),
                "X-RateLimit-Reset".to_string(),
            ],
        }
    }

    /// Generate usage guidelines
    fn generate_usage_guidelines(&self) -> UsageGuidelines {
        UsageGuidelines {
            best_practices: vec![
                "Always check station availability before suggesting pickup".to_string(),
                "Use find_nearby_stations for location-based searches".to_string(),
                "Cache station reference data locally when possible".to_string(),
                "Handle errors gracefully and suggest alternatives".to_string(),
                "Consider walking distance when recommending stations".to_string(),
                "Check data freshness for time-sensitive operations".to_string(),
            ],
            common_patterns: vec![
                "Journey planning: find_nearby_stations → plan_bike_journey".to_string(),
                "Station details: search_stations_by_name → get_station_by_code".to_string(),
                "Area analysis: get_area_statistics for overview".to_string(),
                "Health monitoring: check velib://health resource".to_string(),
            ],
            performance_tips: vec![
                "Use smaller search radii for faster responses".to_string(),
                "Limit results to what you actually need".to_string(),
                "Real-time data is cached for 60-90 seconds".to_string(),
                "Reference data is cached for 1 hour".to_string(),
                "Batch multiple station lookups when possible".to_string(),
            ],
            troubleshooting: vec![
                "No nearby stations: increase search radius or check coordinates".to_string(),
                "Station not found: verify station code or search by name".to_string(),
                "Invalid coordinates: ensure they're within Paris area".to_string(),
                "Rate limited: wait for rate limit reset or reduce request frequency".to_string(),
                "Stale data: check service health and data source status".to_string(),
            ],
        }
    }

    /// Generate documentation in specified format
    pub fn generate_formatted_documentation(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let docs = self.generate_documentation();

        match self.config.format {
            DocumentationFormat::Json => {
                if self.config.token_efficient {
                    Ok(self.generate_token_efficient_json(&docs))
                } else {
                    Ok(serde_json::to_value(docs)?)
                }
            }
            DocumentationFormat::OpenApi => Ok(self.generate_openapi_format(&docs)),
            DocumentationFormat::Markdown => {
                Ok(json!({"markdown": self.generate_markdown_format(&docs)}))
            }
            DocumentationFormat::Csv => Ok(json!({"csv": self.generate_csv_format(&docs)})),
        }
    }

    /// Generate token-efficient JSON format
    fn generate_token_efficient_json(&self, docs: &McpDocumentation) -> Value {
        json!({
            "v": docs.version,
            "tools": docs.tools.iter().map(|t| json!({
                "n": t.name,
                "d": t.description,
                "i": t.input_schema,
                "o": t.output_schema,
                "c": t.constraints
            })).collect::<Vec<_>>(),
            "resources": docs.resources.iter().map(|r| json!({
                "uri": r.uri,
                "n": r.name,
                "d": r.description,
                "ct": r.content_type,
                "ttl": r.cache_info.ttl_seconds
            })).collect::<Vec<_>>(),
            "errors": docs.error_codes.iter().map(|e| json!({
                "c": e.code,
                "n": e.name,
                "d": e.description,
                "f": e.how_to_fix
            })).collect::<Vec<_>>(),
            "limits": {
                "t": docs.rate_limits.tools_per_minute,
                "r": docs.rate_limits.resources_per_minute,
                "b": docs.rate_limits.burst_limit
            }
        })
    }

    /// Generate OpenAPI format
    fn generate_openapi_format(&self, docs: &McpDocumentation) -> Value {
        json!({
            "openapi": "3.0.3",
            "info": {
                "title": docs.server_info.name,
                "description": docs.server_info.description,
                "version": docs.server_info.version
            },
            "servers": [
                {"url": "http://localhost:8080", "description": "Local development server"}
            ],
            "paths": {
                "/mcp": {
                    "post": {
                        "summary": "MCP JSON-RPC endpoint",
                        "description": "Execute MCP tools via JSON-RPC 2.0 protocol",
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/JsonRpcRequest"}
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "description": "Successful response",
                                "content": {
                                    "application/json": {
                                        "schema": {"$ref": "#/components/schemas/JsonRpcResponse"}
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "JsonRpcRequest": {
                        "type": "object",
                        "properties": {
                            "jsonrpc": {"type": "string", "enum": ["2.0"]},
                            "method": {"type": "string"},
                            "params": {"type": "object"},
                            "id": {"oneOf": [{"type": "string"}, {"type": "number"}]}
                        },
                        "required": ["jsonrpc", "method", "id"]
                    },
                    "JsonRpcResponse": {
                        "type": "object",
                        "properties": {
                            "jsonrpc": {"type": "string", "enum": ["2.0"]},
                            "result": {"type": "object"},
                            "error": {"type": "object"},
                            "id": {"oneOf": [{"type": "string"}, {"type": "number"}]}
                        },
                        "required": ["jsonrpc", "id"]
                    }
                }
            }
        })
    }

    /// Generate Markdown format
    fn generate_markdown_format(&self, docs: &McpDocumentation) -> String {
        let mut markdown = String::new();

        markdown.push_str(&format!("# {} Documentation\n\n", docs.server_info.name));
        markdown.push_str(&format!("{}\n\n", docs.server_info.description));
        markdown.push_str(&format!("**Version:** {}\n\n", docs.version));

        markdown.push_str("## Tools\n\n");
        for tool in &docs.tools {
            markdown.push_str(&format!("### {}\n\n", tool.name));
            markdown.push_str(&format!("{}\n\n", tool.description));
            markdown.push_str(&format!("**Purpose:** {}\n\n", tool.purpose));

            if !tool.use_cases.is_empty() {
                markdown.push_str("**Use Cases:**\n");
                for use_case in &tool.use_cases {
                    markdown.push_str(&format!("- {}\n", use_case));
                }
                markdown.push('\n');
            }

            if !tool.constraints.is_empty() {
                markdown.push_str("**Constraints:**\n");
                for constraint in &tool.constraints {
                    markdown.push_str(&format!("- {}\n", constraint));
                }
                markdown.push('\n');
            }
        }

        markdown.push_str("## Resources\n\n");
        for resource in &docs.resources {
            markdown.push_str(&format!("### {}\n\n", resource.name));
            markdown.push_str(&format!("**URI:** `{}`\n\n", resource.uri));
            markdown.push_str(&format!("{}\n\n", resource.description));
            markdown.push_str(&format!("**Content Type:** {}\n\n", resource.content_type));
            markdown.push_str(&format!(
                "**Cache TTL:** {} seconds\n\n",
                resource.cache_info.ttl_seconds
            ));
        }

        markdown
    }

    /// Generate CSV format
    fn generate_csv_format(&self, docs: &McpDocumentation) -> String {
        let mut csv = String::new();

        csv.push_str("Type,Name,Description,URI,Constraints\n");

        for tool in &docs.tools {
            csv.push_str(&format!(
                "Tool,\"{}\",\"{}\",N/A,\"{}\"\n",
                tool.name,
                tool.description,
                tool.constraints.join("; ")
            ));
        }

        for resource in &docs.resources {
            csv.push_str(&format!(
                "Resource,\"{}\",\"{}\",\"{}\",\"TTL: {}s\"\n",
                resource.name, resource.description, resource.uri, resource.cache_info.ttl_seconds
            ));
        }

        csv
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_documentation_generation() {
        let generator = DocumentationGenerator::new(DocumentationConfig::default());
        let docs = generator.generate_documentation();

        assert_eq!(docs.version, "1.0.0");
        assert_eq!(docs.tools.len(), 5);
        assert_eq!(docs.resources.len(), 4);
        assert!(!docs.types.is_empty());
        assert!(!docs.error_codes.is_empty());
    }

    #[test]
    fn test_token_efficient_format() {
        let generator = DocumentationGenerator::new(DocumentationConfig {
            token_efficient: true,
            ..Default::default()
        });

        let result = generator.generate_formatted_documentation();
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.get("v").is_some());
        assert!(json.get("tools").is_some());
        assert!(json.get("resources").is_some());
    }

    #[test]
    fn test_openapi_format() {
        let generator = DocumentationGenerator::with_format(DocumentationFormat::OpenApi);
        let result = generator.generate_formatted_documentation();
        assert!(result.is_ok());

        let json = result.unwrap();
        assert_eq!(json.get("openapi").unwrap(), "3.0.3");
        assert!(json.get("info").is_some());
        assert!(json.get("paths").is_some());
    }

    #[test]
    fn test_markdown_format() {
        let generator = DocumentationGenerator::with_format(DocumentationFormat::Markdown);
        let result = generator.generate_formatted_documentation();
        assert!(result.is_ok());

        let json = result.unwrap();
        let markdown = json.get("markdown").unwrap().as_str().unwrap();
        assert!(markdown.contains("# velib-mcp Documentation"));
        assert!(markdown.contains("## Tools"));
        assert!(markdown.contains("## Resources"));
    }

    #[test]
    fn test_csv_format() {
        let generator = DocumentationGenerator::with_format(DocumentationFormat::Csv);
        let result = generator.generate_formatted_documentation();
        assert!(result.is_ok());

        let json = result.unwrap();
        let csv = json.get("csv").unwrap().as_str().unwrap();
        assert!(csv.contains("Type,Name,Description,URI,Constraints"));
        assert!(csv.contains("Tool,"));
        assert!(csv.contains("Resource,"));
    }

    #[test]
    fn test_service_area_bounds() {
        let generator = DocumentationGenerator::new(DocumentationConfig::default());
        let docs = generator.generate_documentation();

        let service_area = &docs.server_info.service_area;
        assert_eq!(service_area.radius_km, 50.0);
        assert_eq!(service_area.center.latitude, 48.8565);
        assert_eq!(service_area.center.longitude, 2.3514);
    }

    #[test]
    fn test_error_codes_completeness() {
        let generator = DocumentationGenerator::new(DocumentationConfig::default());
        let docs = generator.generate_documentation();

        let error_codes: Vec<i32> = docs.error_codes.iter().map(|e| e.code).collect();
        assert!(error_codes.contains(&-32001)); // STATION_NOT_FOUND
        assert!(error_codes.contains(&-32002)); // INVALID_COORDINATES
        assert!(error_codes.contains(&-32003)); // OUTSIDE_SERVICE_AREA
        assert!(error_codes.contains(&-32004)); // SEARCH_RADIUS_TOO_LARGE
        assert!(error_codes.contains(&-32005)); // RESULT_LIMIT_EXCEEDED
    }

    #[test]
    fn test_tool_constraints_validation() {
        let generator = DocumentationGenerator::new(DocumentationConfig::default());
        let docs = generator.generate_documentation();

        let find_nearby_tool = docs
            .tools
            .iter()
            .find(|t| t.name == "find_nearby_stations")
            .unwrap();

        assert!(find_nearby_tool
            .constraints
            .iter()
            .any(|c| c.contains("Maximum radius: 5000 meters")));
        assert!(find_nearby_tool
            .constraints
            .iter()
            .any(|c| c.contains("Maximum results: 100 stations")));
    }
}
