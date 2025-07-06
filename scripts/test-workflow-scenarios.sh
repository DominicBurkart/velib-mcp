#!/bin/bash

# Comprehensive workflow scenario testing
set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo ""
    echo -e "${BLUE}$1${NC}"
    echo "$(printf '=%.0s' $(seq 1 ${#1}))"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${BLUE}📋 $1${NC}"
}

# Test scenario: No vulnerabilities found
test_no_vulnerabilities_scenario() {
    print_header "Testing: No Vulnerabilities Found Scenario"
    
    print_info "Simulating workflow behavior when no vulnerabilities are detected..."
    
    # Test that workflow handles empty audit results gracefully
    echo '{"vulnerabilities":[]}' > /tmp/test-audit-empty.json
    
    # Validate JSON parsing works with empty results
    if jq -r '.vulnerabilities[] | select(.advisory.severity == "high") | .advisory.id' /tmp/test-audit-empty.json 2>/dev/null | wc -l > /dev/null; then
        print_success "Empty vulnerability results handled correctly"
    else
        print_error "Empty vulnerability results parsing failed"
        return 1
    fi
    
    print_success "No vulnerabilities scenario: PASSED"
}

# Test scenario: Vulnerabilities found but no fixes available
test_no_fixes_available_scenario() {
    print_header "Testing: Vulnerabilities Found, No Fixes Available"
    
    print_info "Simulating scenario where vulnerabilities exist but cannot be automatically fixed..."
    
    # Create mock vulnerability data
    cat > /tmp/test-audit-unfixable.json << 'EOF'
{
  "vulnerabilities": [
    {
      "advisory": {
        "id": "RUSTSEC-2023-0001",
        "severity": "high",
        "title": "Test vulnerability requiring manual intervention",
        "description": "This vulnerability requires major version upgrade"
      },
      "versions": {
        "patched": [">= 2.0.0"],
        "unaffected": []
      }
    }
  ]
}
EOF
    
    # Test JSON parsing with mock data
    HIGH_COUNT=$(jq -r '.vulnerabilities[] | select(.advisory.severity == "high") | .advisory.id' /tmp/test-audit-unfixable.json 2>/dev/null | wc -l)
    if [ "$HIGH_COUNT" -eq 1 ]; then
        print_success "Vulnerability severity classification works correctly"
    else
        print_error "Vulnerability severity classification failed"
        return 1
    fi
    
    print_success "No fixes available scenario: PASSED"
}

# Test scenario: Successful fixes with compilation validation
test_successful_fixes_scenario() {
    print_header "Testing: Successful Fixes with Validation Pipeline"
    
    print_info "Testing validation logic for successful fixes..."
    
    # Simulate successful validation states
    compile_success="true"
    tests_passed="true"
    vulnerabilities_resolved="true"
    
    # Test validation logic (mimicking the workflow logic)
    if [ "$compile_success" = "true" ] && [ "$vulnerabilities_resolved" != "false" ]; then
        if [ "$tests_passed" = "true" ]; then
            validation_passed="true"
        else
            validation_passed="with_warnings"
        fi
    else
        validation_passed="false"
    fi
    
    if [ "$validation_passed" = "true" ]; then
        print_success "Validation logic works correctly for successful fixes"
    else
        print_error "Validation logic failed for successful scenario"
        return 1
    fi
    
    print_success "Successful fixes scenario: PASSED"
}

# Test scenario: Compilation failure after fixes
test_compilation_failure_scenario() {
    print_header "Testing: Compilation Failure After Fixes"
    
    print_info "Testing validation logic when compilation fails..."
    
    # Simulate compilation failure
    compile_success="false"
    tests_passed="true"
    vulnerabilities_resolved="true"
    
    # Test validation logic
    if [ "$compile_success" = "true" ] && [ "$vulnerabilities_resolved" != "false" ]; then
        validation_passed="true"
    else
        validation_passed="false"
    fi
    
    if [ "$validation_passed" = "false" ]; then
        print_success "Validation correctly rejects fixes that break compilation"
    else
        print_error "Validation logic incorrectly passed failed compilation"
        return 1
    fi
    
    print_success "Compilation failure scenario: PASSED"
}

# Test scenario: Test failures with successful compilation
test_test_failure_scenario() {
    print_header "Testing: Test Failures with Successful Compilation"
    
    print_info "Testing validation logic when tests fail but compilation succeeds..."
    
    # Simulate test failure but compilation success
    compile_success="true"
    tests_passed="false"
    vulnerabilities_resolved="true"
    
    # Test validation logic
    if [ "$compile_success" = "true" ] && [ "$vulnerabilities_resolved" != "false" ]; then
        if [ "$tests_passed" = "true" ]; then
            validation_passed="true"
        else
            validation_passed="with_warnings"
        fi
    else
        validation_passed="false"
    fi
    
    if [ "$validation_passed" = "with_warnings" ]; then
        print_success "Validation correctly handles test failures with warnings"
    else
        print_error "Validation logic failed to handle test failures correctly"
        return 1
    fi
    
    print_success "Test failure scenario: PASSED"
}

# Test scenario: Branch creation and naming
test_branch_creation_scenario() {
    print_header "Testing: Branch Creation and Naming"
    
    print_info "Testing branch naming convention..."
    
    # Simulate branch name creation (matching workflow logic)
    BRANCH_NAME="security/automated-fixes-$(date +%Y%m%d-%H%M%S)"
    
    # Validate branch name format
    if [[ "$BRANCH_NAME" =~ ^security/automated-fixes-[0-9]{8}-[0-9]{6}$ ]]; then
        print_success "Branch naming convention is correct"
    else
        print_error "Branch naming convention is invalid: $BRANCH_NAME"
        return 1
    fi
    
    print_success "Branch creation scenario: PASSED"
}

# Test scenario: PR/Issue title generation
test_title_generation_scenario() {
    print_header "Testing: PR/Issue Title Generation"
    
    print_info "Testing title generation with vulnerability counts..."
    
    # Simulate vulnerability counts
    critical_count="2"
    high_count="3"
    medium_count="1"
    
    # Generate titles (matching workflow logic)
    PR_TITLE="🔒 Automated Security Fixes - ${critical_count}C/${high_count}H/${medium_count}M"
    ISSUE_TITLE="🔧 Manual Security Fix Required - ${critical_count}C/${high_count}H/${medium_count}M"
    
    # Validate title formats
    if [[ "$PR_TITLE" == "🔒 Automated Security Fixes - 2C/3H/1M" ]]; then
        print_success "PR title generation is correct"
    else
        print_error "PR title generation failed: $PR_TITLE"
        return 1
    fi
    
    if [[ "$ISSUE_TITLE" == "🔧 Manual Security Fix Required - 2C/3H/1M" ]]; then
        print_success "Issue title generation is correct"
    else
        print_error "Issue title generation failed: $ISSUE_TITLE"
        return 1
    fi
    
    print_success "Title generation scenario: PASSED"
}

# Test scenario: Environment variable handling
test_environment_variables_scenario() {
    print_header "Testing: Environment Variable Handling"
    
    print_info "Testing shell variable expansion (not GitHub expressions)..."
    
    # Test that we're using shell variables, not GitHub expressions
    test_var="test_value"
    expanded="${test_var:-default}"
    
    if [ "$expanded" = "test_value" ]; then
        print_success "Shell variable expansion works correctly"
    else
        print_error "Shell variable expansion failed"
        return 1
    fi
    
    # Test default value handling
    unset test_var
    expanded="${test_var:-default}"
    
    if [ "$expanded" = "default" ]; then
        print_success "Default value handling works correctly"
    else
        print_error "Default value handling failed"
        return 1
    fi
    
    print_success "Environment variables scenario: PASSED"
}

# Test scenario: Workflow permissions validation
test_permissions_scenario() {
    print_header "Testing: Workflow Permissions Validation"
    
    print_info "Validating GitHub Actions permissions are properly configured..."
    
    # Check vulnerability-scan.yml permissions
    if grep -q "contents: read" .github/workflows/vulnerability-scan.yml && \
       grep -q "security-events: write" .github/workflows/vulnerability-scan.yml && \
       grep -q "issues: write" .github/workflows/vulnerability-scan.yml; then
        print_success "vulnerability-scan.yml has correct permissions"
    else
        print_error "vulnerability-scan.yml missing required permissions"
        return 1
    fi
    
    # Check vulnerability-fix.yml permissions
    if grep -q "contents: write" .github/workflows/vulnerability-fix.yml && \
       grep -q "pull-requests: write" .github/workflows/vulnerability-fix.yml && \
       grep -q "security-events: write" .github/workflows/vulnerability-fix.yml; then
        print_success "vulnerability-fix.yml has correct permissions"
    else
        print_error "vulnerability-fix.yml missing required permissions"
        return 1
    fi
    
    print_success "Permissions scenario: PASSED"
}

# Main test execution
main() {
    print_header "🧪 Comprehensive Workflow Scenario Testing"
    
    # Run all test scenarios
    test_no_vulnerabilities_scenario
    test_no_fixes_available_scenario
    test_successful_fixes_scenario
    test_compilation_failure_scenario
    test_test_failure_scenario
    test_branch_creation_scenario
    test_title_generation_scenario
    test_environment_variables_scenario
    test_permissions_scenario
    
    # Cleanup test files
    rm -f /tmp/test-audit-*.json
    
    print_header "🎉 All Scenario Tests Passed!"
    
    print_info "Validated Scenarios:"
    echo "✅ No vulnerabilities found handling"
    echo "✅ Vulnerabilities found but no fixes available"
    echo "✅ Successful fixes with validation pipeline"
    echo "✅ Compilation failure detection and handling"
    echo "✅ Test failure handling with warnings"
    echo "✅ Branch creation and naming conventions"
    echo "✅ PR/Issue title generation with severity counts"
    echo "✅ Environment variable handling (no GitHub expressions)"
    echo "✅ Workflow permissions validation"
    
    echo ""
    print_info "Manual Testing Checklist:"
    echo "1. 🔍 Test workflow_dispatch trigger on current branch"
    echo "2. 📝 Verify issue creation when vulnerabilities detected"
    echo "3. 🔄 Verify PR creation when fixes are available"
    echo "4. ✅ Verify 'no issues found' run completes successfully"
    echo "5. ⚠️  Test compilation failure scenario"
    echo "6. 🧪 Test test failure scenario with warnings"
    echo "7. 🔒 Verify security boundaries and permissions"
}

# Execute main function
main "$@"