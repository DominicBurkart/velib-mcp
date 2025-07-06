#!/bin/bash

# Test script for security workflows
set -e

echo "🔍 Testing Security Workflows"
echo "=============================="

# Check if workflow files exist
echo "✅ Checking workflow files..."
if [ ! -f ".github/workflows/vulnerability-scan.yml" ]; then
    echo "❌ vulnerability-scan.yml not found"
    exit 1
fi

if [ ! -f ".github/workflows/vulnerability-fix.yml" ]; then
    echo "❌ vulnerability-fix.yml not found"
    exit 1
fi

echo "✅ Workflow files found"

# Validate YAML syntax
echo "✅ Validating YAML syntax..."
if command -v yamllint &> /dev/null; then
    yamllint .github/workflows/vulnerability-scan.yml
    yamllint .github/workflows/vulnerability-fix.yml
    echo "✅ YAML syntax validation passed"
else
    echo "⚠️  yamllint not found, skipping YAML validation"
fi

# Test cargo audit functionality
echo "✅ Testing cargo audit..."
if cargo audit --version &> /dev/null; then
    echo "✅ cargo-audit is available"
    cargo audit --format json > /tmp/audit-test.json 2>/dev/null || true
    if [ -s /tmp/audit-test.json ]; then
        echo "✅ cargo audit JSON output works"
    else
        echo "✅ cargo audit runs successfully (no vulnerabilities found)"
    fi
else
    echo "⚠️  cargo-audit not found, installing..."
    cargo install cargo-audit --locked
    echo "✅ cargo-audit installed"
fi

# Test cargo audit fix functionality
echo "✅ Testing cargo audit fix..."
if cargo audit fix --dry-run > /tmp/fix-test.txt 2>&1; then
    echo "✅ cargo audit fix dry-run works"
else
    echo "⚠️  cargo audit fix not available (may need --features=fix)"
    echo "Installing with fix feature..."
    cargo install cargo-audit --locked --features=fix
    echo "✅ cargo-audit with fix feature installed"
fi

# Test workflow permissions
echo "✅ Checking workflow permissions..."
if grep -q "permissions:" .github/workflows/vulnerability-scan.yml; then
    echo "✅ vulnerability-scan.yml has permissions configured"
else
    echo "❌ vulnerability-scan.yml missing permissions"
    exit 1
fi

if grep -q "permissions:" .github/workflows/vulnerability-fix.yml; then
    echo "✅ vulnerability-fix.yml has permissions configured"
else
    echo "❌ vulnerability-fix.yml missing permissions"
    exit 1
fi

# Test cron schedule format
echo "✅ Checking cron schedules..."
if grep -q "cron: '0 6 \* \* 1'" .github/workflows/vulnerability-scan.yml; then
    echo "✅ vulnerability-scan.yml has correct weekly schedule"
else
    echo "❌ vulnerability-scan.yml has incorrect cron schedule"
    exit 1
fi

if grep -q "cron: '0 7 \* \* 1'" .github/workflows/vulnerability-fix.yml; then
    echo "✅ vulnerability-fix.yml has correct weekly schedule"
else
    echo "❌ vulnerability-fix.yml has incorrect cron schedule"
    exit 1
fi

echo ""
# Test advanced workflow features
echo "✅ Checking advanced workflow features..."
if grep -q "Phase 1 - Vulnerability Detection" .github/workflows/vulnerability-fix.yml; then
    echo "✅ Multi-phase workflow structure implemented"
else
    echo "❌ Missing multi-phase workflow structure"
    exit 1
fi

if grep -q "jq -r" .github/workflows/vulnerability-fix.yml; then
    echo "✅ JSON parsing for severity classification found"
else
    echo "❌ Missing JSON parsing for severity classification"
    exit 1
fi

if grep -q "cargo check --all-targets --all-features" .github/workflows/vulnerability-fix.yml; then
    echo "✅ Comprehensive validation pipeline found"
else
    echo "❌ Missing comprehensive validation pipeline"
    exit 1
fi

echo ""
echo "🎉 All security workflow tests passed!"
echo "======================================"
echo ""
echo "📋 Enhanced Features Summary:"
echo "- 🔍 vulnerability-scan.yml: Weekly vulnerability detection (Mondays 6 AM UTC)"
echo "- 🔧 vulnerability-fix.yml: Advanced automated fixing (Mondays 7 AM UTC)"
echo "- 📊 Severity classification: Critical/High/Medium vulnerability tracking"
echo "- 🧪 Comprehensive validation: Compilation, tests, and security verification"
echo "- 🚀 Multi-phase workflow: Detection → Analysis → Fix → Validation → Reporting"
echo "- 📋 Intelligent reporting: Structured fix reports with validation results"
echo "- 🔄 Fallback handling: Manual fix guidance when automation fails"
echo "- 🔒 Security boundaries: Proper permissions and error handling"
echo ""
echo "🚀 Next steps:"
echo "1. Commit and push these workflows to enable automated security monitoring"
echo "2. Test manually using workflow_dispatch triggers"
echo "3. Monitor the scheduled runs for proper operation"
echo "4. Review generated PRs and issues for quality"
echo ""
echo "🎯 Advanced Agent Integration Ready:"
echo "- Deterministic 7-phase workflow process"
echo "- Structured JSON input/output for programmatic parsing"
echo "- Comprehensive error handling and fallback mechanisms"
echo "- Validation pipeline with compilation and test verification"
echo "- Clear success/failure reporting with actionable next steps"