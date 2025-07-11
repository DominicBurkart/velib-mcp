use serde_json::json;
use velib_mcp::mcp::documentation::{
    DocumentationConfig, DocumentationFormat, DocumentationGenerator,
};

#[tokio::test]
async fn test_docs_schema_json_format() {
    let generator = DocumentationGenerator::new(DocumentationConfig {
        format: DocumentationFormat::Json,
        include_examples: true,
        token_efficient: false,
        ..Default::default()
    });

    let result = generator.generate_formatted_documentation();
    assert!(result.is_ok());

    let docs = result.unwrap();
    assert!(docs.is_object());
    assert!(docs.get("version").is_some());
    assert!(docs.get("server_info").is_some());
    assert!(docs.get("tools").is_some());
    assert!(docs.get("resources").is_some());
    assert!(docs.get("types").is_some());
    assert!(docs.get("error_codes").is_some());
}

#[tokio::test]
async fn test_docs_schema_openapi_format() {
    let generator = DocumentationGenerator::new(DocumentationConfig {
        format: DocumentationFormat::OpenApi,
        include_examples: false,
        ..Default::default()
    });

    let result = generator.generate_formatted_documentation();
    assert!(result.is_ok());

    let docs = result.unwrap();
    assert_eq!(docs.get("openapi").unwrap(), "3.0.3");
    assert!(docs.get("info").is_some());
    assert!(docs.get("paths").is_some());
    assert!(docs.get("components").is_some());
}

#[tokio::test]
async fn test_docs_schema_token_efficient() {
    let generator = DocumentationGenerator::new(DocumentationConfig {
        format: DocumentationFormat::Json,
        token_efficient: true,
        include_examples: false,
        ..Default::default()
    });

    let result = generator.generate_formatted_documentation();
    assert!(result.is_ok());

    let docs = result.unwrap();

    // Check token-efficient format (shortened keys)
    assert!(docs.get("v").is_some()); // version
    assert!(docs.get("tools").is_some());
    assert!(docs.get("resources").is_some());
    assert!(docs.get("errors").is_some());
    assert!(docs.get("limits").is_some());
}

#[tokio::test]
async fn test_docs_schema_markdown_format() {
    let generator = DocumentationGenerator::new(DocumentationConfig {
        format: DocumentationFormat::Markdown,
        ..Default::default()
    });

    let result = generator.generate_formatted_documentation();
    assert!(result.is_ok());

    let docs = result.unwrap();
    let markdown = docs.get("markdown").unwrap().as_str().unwrap();

    assert!(markdown.contains("# velib-mcp Documentation"));
    assert!(markdown.contains("## Tools"));
    assert!(markdown.contains("## Resources"));
    assert!(markdown.contains("find_nearby_stations"));
    assert!(markdown.contains("velib://"));
}

#[tokio::test]
async fn test_docs_schema_csv_format() {
    let generator = DocumentationGenerator::new(DocumentationConfig {
        format: DocumentationFormat::Csv,
        ..Default::default()
    });

    let result = generator.generate_formatted_documentation();
    assert!(result.is_ok());

    let docs = result.unwrap();
    let csv = docs.get("csv").unwrap().as_str().unwrap();

    assert!(csv.contains("Type,Name,Description,URI,Constraints"));
    assert!(csv.contains("Tool,"));
    assert!(csv.contains("Resource,"));
    assert!(csv.contains("find_nearby_stations"));
    assert!(csv.contains("velib://"));
}

#[tokio::test]
async fn test_documentation_content_completeness() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    // Check server info
    assert_eq!(docs.server_info.name, "velib-mcp");
    assert!(docs.server_info.description.contains("Paris Velib"));
    assert_eq!(docs.server_info.service_area.radius_km, 50.0);
    assert_eq!(docs.server_info.service_area.center.latitude, 48.8565);
    assert_eq!(docs.server_info.service_area.center.longitude, 2.3514);

    // Check tools
    assert_eq!(docs.tools.len(), 5);
    let tool_names: Vec<&String> = docs.tools.iter().map(|t| &t.name).collect();
    assert!(tool_names.contains(&&"find_nearby_stations".to_string()));
    assert!(tool_names.contains(&&"get_station_by_code".to_string()));
    assert!(tool_names.contains(&&"search_stations_by_name".to_string()));
    assert!(tool_names.contains(&&"get_area_statistics".to_string()));
    assert!(tool_names.contains(&&"plan_bike_journey".to_string()));

    // Check resources
    assert_eq!(docs.resources.len(), 4);
    let resource_uris: Vec<&String> = docs.resources.iter().map(|r| &r.uri).collect();
    assert!(resource_uris.contains(&&"velib://stations/reference".to_string()));
    assert!(resource_uris.contains(&&"velib://stations/realtime".to_string()));
    assert!(resource_uris.contains(&&"velib://stations/complete".to_string()));
    assert!(resource_uris.contains(&&"velib://health".to_string()));

    // Check error codes
    assert!(docs.error_codes.len() >= 5);
    let error_codes: Vec<i32> = docs.error_codes.iter().map(|e| e.code).collect();
    assert!(error_codes.contains(&-32001)); // STATION_NOT_FOUND
    assert!(error_codes.contains(&-32002)); // INVALID_COORDINATES
    assert!(error_codes.contains(&-32003)); // OUTSIDE_SERVICE_AREA

    // Check types
    assert!(docs.types.contains_key("Coordinates"));
    assert!(docs.types.contains_key("BikeAvailability"));
    assert!(docs.types.contains_key("StationStatus"));
    assert!(docs.types.contains_key("DataFreshness"));
    assert!(docs.types.contains_key("BikeTypeFilter"));

    // Check usage guidelines
    assert!(!docs.usage_guidelines.best_practices.is_empty());
    assert!(!docs.usage_guidelines.common_patterns.is_empty());
    assert!(!docs.usage_guidelines.performance_tips.is_empty());
    assert!(!docs.usage_guidelines.troubleshooting.is_empty());
}

#[tokio::test]
async fn test_tool_documentation_examples() {
    let generator = DocumentationGenerator::new(DocumentationConfig {
        include_examples: true,
        ..Default::default()
    });
    let docs = generator.generate_documentation();

    // Check that examples are included
    for tool in &docs.tools {
        if !tool.examples.is_empty() {
            let example = &tool.examples[0];
            assert!(!example.title.is_empty());
            assert!(!example.description.is_empty());
            assert!(example.input.is_object());
            assert!(example.output.is_object());
        }
    }
}

#[tokio::test]
async fn test_tool_documentation_constraints() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    // Check find_nearby_stations constraints
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
    assert!(find_nearby_tool
        .constraints
        .iter()
        .any(|c| c.contains("Paris metropolitan area")));

    // Check performance notes
    assert!(find_nearby_tool
        .performance_notes
        .iter()
        .any(|n| n.contains("response time")));
    assert!(find_nearby_tool
        .performance_notes
        .iter()
        .any(|n| n.contains("sorted by distance")));
}

#[tokio::test]
async fn test_error_documentation_completeness() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    for error in &docs.error_codes {
        assert!(error.code < 0); // JSON-RPC error codes should be negative
        assert!(!error.name.is_empty());
        assert!(!error.description.is_empty());
        assert!(!error.when_occurs.is_empty());
        assert!(!error.how_to_fix.is_empty());
        assert!(error.example.is_object());

        // Check that error example has proper structure
        let example = error.example.as_object().unwrap();
        assert!(example.contains_key("error"));
        let error_obj = example.get("error").unwrap().as_object().unwrap();
        assert!(error_obj.contains_key("code"));
        assert!(error_obj.contains_key("message"));
        assert_eq!(
            error_obj.get("code").unwrap().as_i64().unwrap(),
            error.code as i64
        );
    }
}

#[tokio::test]
async fn test_type_documentation_schemas() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    // Check Coordinates type
    let coordinates_type = docs.types.get("Coordinates").unwrap();
    assert!(coordinates_type.schema.is_object());
    assert!(coordinates_type.schema.get("type").unwrap() == "object");
    assert!(coordinates_type.schema.get("properties").is_some());
    assert!(!coordinates_type.examples.is_empty());
    assert!(!coordinates_type.validation_rules.is_empty());

    // Check BikeAvailability type
    let bike_availability_type = docs.types.get("BikeAvailability").unwrap();
    assert!(bike_availability_type.schema.is_object());
    let properties = bike_availability_type
        .schema
        .get("properties")
        .unwrap()
        .as_object()
        .unwrap();
    assert!(properties.contains_key("mechanical"));
    assert!(properties.contains_key("electric"));

    // Check StationStatus enum
    let station_status_type = docs.types.get("StationStatus").unwrap();
    assert!(station_status_type.schema.is_object());
    let enum_values = station_status_type
        .schema
        .get("enum")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(enum_values.contains(&json!("Open")));
    assert!(enum_values.contains(&json!("Closed")));
    assert!(enum_values.contains(&json!("Maintenance")));
}

#[tokio::test]
async fn test_usage_guidelines_content() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    let guidelines = &docs.usage_guidelines;

    // Check best practices
    assert!(guidelines
        .best_practices
        .iter()
        .any(|p| p.contains("station availability")));
    assert!(guidelines
        .best_practices
        .iter()
        .any(|p| p.contains("find_nearby_stations")));

    // Check common patterns
    assert!(guidelines
        .common_patterns
        .iter()
        .any(|p| p.contains("plan_bike_journey")));
    assert!(guidelines
        .common_patterns
        .iter()
        .any(|p| p.contains("search_stations_by_name")));

    // Check performance tips
    assert!(guidelines
        .performance_tips
        .iter()
        .any(|t| t.contains("search radii")));
    assert!(guidelines
        .performance_tips
        .iter()
        .any(|t| t.contains("cached")));

    // Check troubleshooting
    assert!(guidelines
        .troubleshooting
        .iter()
        .any(|t| t.contains("No nearby stations")));
    assert!(guidelines
        .troubleshooting
        .iter()
        .any(|t| t.contains("Invalid coordinates")));
}

#[tokio::test]
async fn test_resource_documentation_cache_info() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    // Check reference data cache
    let reference_resource = docs
        .resources
        .iter()
        .find(|r| r.uri == "velib://stations/reference")
        .unwrap();
    assert_eq!(reference_resource.cache_info.ttl_seconds, 3600);
    assert_eq!(
        reference_resource.cache_info.update_frequency,
        "Daily at 06:00 UTC"
    );

    // Check real-time data cache
    let realtime_resource = docs
        .resources
        .iter()
        .find(|r| r.uri == "velib://stations/realtime")
        .unwrap();
    assert_eq!(realtime_resource.cache_info.ttl_seconds, 90);
    assert_eq!(
        realtime_resource.cache_info.update_frequency,
        "Every 60 seconds"
    );
}

#[tokio::test]
async fn test_rate_limits_documentation() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    let rate_limits = &docs.rate_limits;
    assert_eq!(rate_limits.tools_per_minute, 100);
    assert_eq!(rate_limits.resources_per_minute, 60);
    assert_eq!(rate_limits.burst_limit, 10);
    assert!(rate_limits
        .headers
        .contains(&"X-RateLimit-Limit".to_string()));
    assert!(rate_limits
        .headers
        .contains(&"X-RateLimit-Remaining".to_string()));
    assert!(rate_limits
        .headers
        .contains(&"X-RateLimit-Reset".to_string()));
}

#[tokio::test]
async fn test_llm_friendly_features() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    // Check that tools have LLM-friendly descriptions
    for tool in &docs.tools {
        assert!(!tool.description.is_empty());
        assert!(!tool.purpose.is_empty());
        assert!(!tool.use_cases.is_empty());
        assert!(!tool.constraints.is_empty());
        assert!(!tool.performance_notes.is_empty());

        // Check input/output schemas are present
        assert!(tool.input_schema.is_object());
        assert!(tool.output_schema.is_object());
    }

    // Check that error codes have actionable guidance
    for error in &docs.error_codes {
        assert!(error.how_to_fix.len() > 20); // Should be detailed guidance
        assert!(error.when_occurs.len() > 10); // Should explain when it occurs
    }
}

#[tokio::test]
async fn test_service_area_documentation() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    let service_area = &docs.server_info.service_area;
    assert_eq!(service_area.radius_km, 50.0);
    assert_eq!(service_area.center.latitude, 48.8565); // Paris City Hall
    assert_eq!(service_area.center.longitude, 2.3514);
    assert!(service_area.description.contains("Paris metropolitan area"));
    assert!(service_area.description.contains("50km"));

    // Check geographic bounds
    assert_eq!(service_area.bounds.north, 49.0);
    assert_eq!(service_area.bounds.south, 48.7);
    assert_eq!(service_area.bounds.east, 2.6);
    assert_eq!(service_area.bounds.west, 2.0);
}

#[tokio::test]
async fn test_data_sources_documentation() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    let data_sources = &docs.server_info.data_sources;
    assert_eq!(data_sources.len(), 2);

    // Check real-time data source
    let realtime_source = data_sources
        .iter()
        .find(|s| s.name.contains("Real-time"))
        .unwrap();
    assert_eq!(realtime_source.update_frequency, "60 seconds");
    assert_eq!(realtime_source.cache_ttl_seconds, 90);
    assert!(realtime_source.description.contains("Live bike"));

    // Check reference data source
    let reference_source = data_sources
        .iter()
        .find(|s| s.name.contains("Reference"))
        .unwrap();
    assert_eq!(reference_source.update_frequency, "Daily at 06:00 UTC");
    assert_eq!(reference_source.cache_ttl_seconds, 3600);
    assert!(reference_source.description.contains("Station locations"));
}

#[tokio::test]
async fn test_server_capabilities_documentation() {
    let generator = DocumentationGenerator::new(DocumentationConfig::default());
    let docs = generator.generate_documentation();

    let capabilities = &docs.server_info.capabilities;
    assert!(capabilities.contains(&"Real-time bike availability".to_string()));
    assert!(capabilities.contains(&"Station location search".to_string()));
    assert!(capabilities.contains(&"Journey planning".to_string()));
    assert!(capabilities.contains(&"Area statistics".to_string()));
    assert!(capabilities.contains(&"Geographic filtering".to_string()));
}
