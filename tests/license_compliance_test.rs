use std::process::Command;
use std::fs;
use std::path::Path;

/// Test that cargo-deny configuration exists and is valid
#[test]
fn test_cargo_deny_config_exists() {
    let config_path = Path::new("deny.toml");
    assert!(config_path.exists(), "deny.toml configuration file should exist");
    
    // Verify the file is valid TOML
    let content = fs::read_to_string(config_path)
        .expect("Should be able to read deny.toml");
    
    let parsed: toml::Value = toml::from_str(&content)
        .expect("deny.toml should be valid TOML");
    
    // Verify required sections exist
    assert!(parsed.get("licenses").is_some(), "licenses section should exist");
    assert!(parsed.get("bans").is_some(), "bans section should exist");
    assert!(parsed.get("advisories").is_some(), "advisories section should exist");
}

/// Test that cargo-deny check passes for licenses
#[test]
fn test_license_compliance_check() {
    let output = Command::new("cargo")
        .args(&["deny", "check", "licenses"])
        .output()
        .expect("cargo-deny should be installed and runnable");
    
    if !output.status.success() {
        eprintln!("cargo-deny output: {}", String::from_utf8_lossy(&output.stderr));
        panic!("License compliance check failed");
    }
}

/// Test that cargo-deny check passes for banned dependencies
#[test]
fn test_banned_dependencies_check() {
    let output = Command::new("cargo")
        .args(&["deny", "check", "bans"])
        .output()
        .expect("cargo-deny should be installed and runnable");
    
    if !output.status.success() {
        eprintln!("cargo-deny output: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Banned dependencies check failed");
    }
}

/// Test that cargo-deny check passes for security advisories
#[test]
fn test_security_advisories_check() {
    let output = Command::new("cargo")
        .args(&["deny", "check", "advisories"])
        .output()
        .expect("cargo-deny should be installed and runnable");
    
    if !output.status.success() {
        eprintln!("cargo-deny output: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Security advisories check failed");
    }
}

/// Test that all our dependencies have compatible licenses
#[test]
fn test_dual_license_compatibility() {
    // This test verifies that our dual MIT OR Apache-2.0 licensing
    // is compatible with all dependencies
    let output = Command::new("cargo")
        .args(&["deny", "check", "licenses"])
        .output()
        .expect("cargo-deny should be installed and runnable");
    
    if !output.status.success() {
        eprintln!("cargo-deny licenses output: {}", String::from_utf8_lossy(&output.stderr));
        panic!("License compatibility check failed");
    }
}

/// Test that cargo-deny configuration allows expected licenses
#[test]
fn test_allowed_licenses_configuration() {
    let config_path = Path::new("deny.toml");
    let content = fs::read_to_string(config_path)
        .expect("Should be able to read deny.toml");
    
    let parsed: toml::Value = toml::from_str(&content)
        .expect("deny.toml should be valid TOML");
    
    let licenses = parsed.get("licenses").unwrap();
    let allow = licenses.get("allow").unwrap().as_array()
        .expect("licenses.allow should be an array");
    
    // Check that our expected licenses are in the allow list
    let allowed_licenses: Vec<&str> = allow.iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    
    assert!(allowed_licenses.contains(&"MIT"), "MIT license should be allowed");
    assert!(allowed_licenses.contains(&"Apache-2.0"), "Apache-2.0 license should be allowed");
    assert!(allowed_licenses.contains(&"BSD-3-Clause"), "BSD-3-Clause license should be allowed");
    assert!(allowed_licenses.contains(&"ISC"), "ISC license should be allowed");
}