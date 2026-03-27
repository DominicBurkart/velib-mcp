//! Tests for GeographicBounds::contains and serialization.

use velib_mcp::Coordinates;
use velib_mcp::mcp::types::GeographicBounds;

fn paris_bounds() -> GeographicBounds {
    GeographicBounds {
        north: 48.90,
        south: 48.82,
        east: 2.42,
        west: 2.25,
    }
}

#[test]
fn contains_point_inside_bounds() {
    let bounds = paris_bounds();
    let inside = Coordinates::new(48.86, 2.35);
    assert!(bounds.contains(&inside));
}

#[test]
fn rejects_point_north_of_bounds() {
    let bounds = paris_bounds();
    let north = Coordinates::new(48.91, 2.35);
    assert!(!bounds.contains(&north));
}

#[test]
fn rejects_point_south_of_bounds() {
    let bounds = paris_bounds();
    let south = Coordinates::new(48.81, 2.35);
    assert!(!bounds.contains(&south));
}

#[test]
fn rejects_point_east_of_bounds() {
    let bounds = paris_bounds();
    let east = Coordinates::new(48.86, 2.43);
    assert!(!bounds.contains(&east));
}

#[test]
fn rejects_point_west_of_bounds() {
    let bounds = paris_bounds();
    let west = Coordinates::new(48.86, 2.24);
    assert!(!bounds.contains(&west));
}

#[test]
fn contains_point_on_north_boundary() {
    let bounds = paris_bounds();
    let on_edge = Coordinates::new(48.90, 2.35);
    assert!(bounds.contains(&on_edge), "Points on the boundary should be included");
}

#[test]
fn contains_point_on_south_boundary() {
    let bounds = paris_bounds();
    let on_edge = Coordinates::new(48.82, 2.35);
    assert!(bounds.contains(&on_edge));
}

#[test]
fn contains_point_on_east_boundary() {
    let bounds = paris_bounds();
    let on_edge = Coordinates::new(48.86, 2.42);
    assert!(bounds.contains(&on_edge));
}

#[test]
fn contains_point_on_west_boundary() {
    let bounds = paris_bounds();
    let on_edge = Coordinates::new(48.86, 2.25);
    assert!(bounds.contains(&on_edge));
}

#[test]
fn contains_all_four_corners() {
    let bounds = paris_bounds();
    assert!(bounds.contains(&Coordinates::new(48.90, 2.25))); // NW
    assert!(bounds.contains(&Coordinates::new(48.90, 2.42))); // NE
    assert!(bounds.contains(&Coordinates::new(48.82, 2.25))); // SW
    assert!(bounds.contains(&Coordinates::new(48.82, 2.42))); // SE
}

#[test]
fn geographic_bounds_serialization_roundtrip() {
    let bounds = paris_bounds();
    let json = serde_json::to_string(&bounds).unwrap();
    let deserialized: GeographicBounds = serde_json::from_str(&json).unwrap();

    assert!((deserialized.north - bounds.north).abs() < f64::EPSILON);
    assert!((deserialized.south - bounds.south).abs() < f64::EPSILON);
    assert!((deserialized.east - bounds.east).abs() < f64::EPSILON);
    assert!((deserialized.west - bounds.west).abs() < f64::EPSILON);
}
