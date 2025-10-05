#!/usr/bin/env bash
set -euo pipefail

# check-features.sh - Validate language plugin feature flag configuration
#
# Ensures all language plugins are correctly registered in:
# 1. crates/cb-services/src/services/registry_builder.rs
# 2. Root Cargo.toml [features] and [workspace.dependencies]
# 3. crates/cb-handlers/Cargo.toml [dependencies] and [features]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

REGISTRY_FILE="$WORKSPACE_ROOT/crates/cb-services/src/services/registry_builder.rs"
ROOT_CARGO="$WORKSPACE_ROOT/Cargo.toml"
HANDLERS_CARGO="$WORKSPACE_ROOT/crates/cb-handlers/Cargo.toml"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

ERROR_COUNT=0
WARNINGS=()

echo -e "${BLUE}Checking language plugin feature flag configuration...${NC}\n"

# Find all language plugins
PLUGINS=$(find "$SCRIPT_DIR" -maxdepth 1 -type d -name "cb-lang-*" | sort)

if [ -z "$PLUGINS" ]; then
    echo -e "${YELLOW}No language plugins found in ${SCRIPT_DIR}${NC}"
    exit 0
fi

# Check each plugin
for PLUGIN_DIR in $PLUGINS; do
    PLUGIN_NAME=$(basename "$PLUGIN_DIR")
    LANG_NAME=$(echo "$PLUGIN_NAME" | sed 's/cb-lang-//')
    FEATURE_NAME="lang-${LANG_NAME}"

    echo -e "${BLUE}Checking ${PLUGIN_NAME}...${NC}"

    PLUGIN_OK=true
    ISSUES=()

    # Check 1: Registry builder
    # Use grep -A to check feature flag and registration are together
    if grep -A 5 "#\[cfg(feature = \"${FEATURE_NAME}\")\]" "$REGISTRY_FILE" | \
       grep -q "${PLUGIN_NAME}::"; then
        echo -e "  ${GREEN}✓${NC} Registered in registry_builder.rs"
    else
        echo -e "  ${RED}✗${NC} MISSING from registry_builder.rs"
        ISSUES+=("Add registration in $REGISTRY_FILE")
        PLUGIN_OK=false
    fi

    # Check 2: Root Cargo.toml feature flag
    if grep -q "^${FEATURE_NAME} = " "$ROOT_CARGO" || \
       grep -q "\"${FEATURE_NAME}\"" "$ROOT_CARGO"; then
        echo -e "  ${GREEN}✓${NC} Feature flag in root Cargo.toml"
    else
        echo -e "  ${RED}✗${NC} MISSING feature flag in root Cargo.toml"
        ISSUES+=("Add '${FEATURE_NAME} = [\"${PLUGIN_NAME}\"]' to [features] in $ROOT_CARGO")
        PLUGIN_OK=false
    fi

    # Check 3: Root Cargo.toml workspace dependencies
    if grep -q "^${PLUGIN_NAME} = " "$ROOT_CARGO"; then
        echo -e "  ${GREEN}✓${NC} Workspace dependency in root Cargo.toml"
    else
        echo -e "  ${RED}✗${NC} MISSING from [workspace.dependencies] in root Cargo.toml"
        ISSUES+=("Add '${PLUGIN_NAME} = { path = \"crates/languages/${PLUGIN_NAME}\" }' to [workspace.dependencies] in $ROOT_CARGO")
        PLUGIN_OK=false
    fi

    # Check 4: cb-handlers Cargo.toml dependency
    if grep -q "^${PLUGIN_NAME} = .*optional = true" "$HANDLERS_CARGO"; then
        echo -e "  ${GREEN}✓${NC} Optional dependency in cb-handlers/Cargo.toml"
    elif grep -q "^# ${PLUGIN_NAME} = " "$HANDLERS_CARGO"; then
        echo -e "  ${YELLOW}⚠${NC}  COMMENTED OUT in cb-handlers/Cargo.toml"
        WARNINGS+=("${PLUGIN_NAME}: dependency is commented out in $HANDLERS_CARGO")
        PLUGIN_OK=false
    else
        echo -e "  ${RED}✗${NC} MISSING from cb-handlers/Cargo.toml dependencies"
        ISSUES+=("Add '${PLUGIN_NAME} = { workspace = true, optional = true }' to [dependencies] in $HANDLERS_CARGO")
        PLUGIN_OK=false
    fi

    # Check 5: cb-handlers Cargo.toml feature
    if grep -q "^${FEATURE_NAME} = .*\"dep:${PLUGIN_NAME}\"" "$HANDLERS_CARGO" || \
       grep -q "^${FEATURE_NAME} = .*\"${PLUGIN_NAME}\"" "$HANDLERS_CARGO"; then
        echo -e "  ${GREEN}✓${NC} Feature in cb-handlers/Cargo.toml"
    elif grep -q "^# ${FEATURE_NAME} = " "$HANDLERS_CARGO"; then
        echo -e "  ${YELLOW}⚠${NC}  COMMENTED OUT feature in cb-handlers/Cargo.toml"
        WARNINGS+=("${PLUGIN_NAME}: feature is commented out in $HANDLERS_CARGO")
        PLUGIN_OK=false
    else
        echo -e "  ${RED}✗${NC} MISSING feature in cb-handlers/Cargo.toml"
        ISSUES+=("Add '${FEATURE_NAME} = [\"dep:${PLUGIN_NAME}\"]' to [features] in $HANDLERS_CARGO")
        PLUGIN_OK=false
    fi

    # Check 6: Verify plugin crate builds
    if [ -f "$PLUGIN_DIR/Cargo.toml" ]; then
        echo -e "  ${GREEN}✓${NC} Cargo.toml exists"
    else
        echo -e "  ${RED}✗${NC} MISSING Cargo.toml"
        ISSUES+=("Create Cargo.toml in $PLUGIN_DIR")
        PLUGIN_OK=false
    fi

    # Check 7: Verify lib.rs exists
    if [ -f "$PLUGIN_DIR/src/lib.rs" ]; then
        echo -e "  ${GREEN}✓${NC} src/lib.rs exists"
    else
        echo -e "  ${RED}✗${NC} MISSING src/lib.rs"
        ISSUES+=("Create src/lib.rs in $PLUGIN_DIR")
        PLUGIN_OK=false
    fi

    if [ "$PLUGIN_OK" = true ]; then
        echo -e "  ${GREEN}✓ All checks passed${NC}\n"
    else
        echo -e "  ${RED}✗ Configuration issues detected${NC}\n"

        if [ ${#ISSUES[@]} -gt 0 ]; then
            echo -e "  ${YELLOW}Required fixes for ${PLUGIN_NAME}:${NC}"
            for issue in "${ISSUES[@]}"; do
                echo -e "    ${RED}•${NC} $issue"
            done
            echo ""
        fi

        ((ERROR_COUNT++))
    fi
done

# Print summary
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

if [ ${#WARNINGS[@]} -gt 0 ]; then
    echo -e "\n${YELLOW}Warnings:${NC}"
    for warning in "${WARNINGS[@]}"; do
        echo -e "  ${YELLOW}⚠${NC}  $warning"
    done
    echo ""
fi

if [ $ERROR_COUNT -eq 0 ]; then
    echo -e "${GREEN}✓ All language plugins are correctly configured!${NC}\n"
    exit 0
else
    echo -e "${RED}✗ Found $ERROR_COUNT plugin(s) with configuration issues${NC}\n"
    echo -e "${YELLOW}Tips:${NC}"
    echo -e "  • Run ${GREEN}./crates/languages/new-lang.sh <language>${NC} to scaffold new plugins"
    echo -e "  • See ${GREEN}crates/languages/README.md${NC} for manual setup instructions"
    echo -e "  • Check reference plugins: ${GREEN}cb-lang-rust, cb-lang-go, cb-lang-typescript${NC}"
    echo ""
    exit 1
fi
