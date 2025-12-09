#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}  AvocadoDB Version Bump Utility${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# Function to get current version
get_current_version() {
    if [ -f "$PROJECT_ROOT/version.txt" ]; then
        cat "$PROJECT_ROOT/version.txt"
    else
        echo "0.0.0"
    fi
}

# Function to parse semver
parse_version() {
    local version=$1
    local major minor patch prerelease
    
    # Extract major.minor.patch
    if [[ "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-(.+))?$ ]]; then
        major="${BASH_REMATCH[1]}"
        minor="${BASH_REMATCH[2]}"
        patch="${BASH_REMATCH[3]}"
        prerelease="${BASH_REMATCH[5]}"
    else
        echo -e "${RED}Error: Invalid version format${NC}"
        exit 1
    fi
    
    echo "$major $minor $patch $prerelease"
}

# Function to increment version
increment_version() {
    local bump_type=$1
    local current_version=$2
    
    read -r major minor patch prerelease <<< "$(parse_version "$current_version")"
    
    case "$bump_type" in
        major)
            major=$((major + 1))
            minor=0
            patch=0
            prerelease=""
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            prerelease=""
            ;;
        patch)
            patch=$((patch + 1))
            prerelease=""
            ;;
        *)
            echo -e "${RED}Error: Invalid bump type${NC}"
            exit 1
            ;;
    esac
    
    echo "${major}.${minor}.${patch}"
}

# Get current version
CURRENT_VERSION=$(get_current_version)
echo -e "Current version: ${GREEN}${CURRENT_VERSION}${NC}"
echo ""

# Determine bump type
echo "What type of version bump?"
echo "1. Major (X.0.0) - Breaking changes"
echo "2. Minor (0.X.0) - New features, backward compatible"
echo "3. Patch (0.0.X) - Bug fixes"
echo "4. Custom - Enter version manually"
echo ""
read -p "Enter your choice (1-4): " -n 1 -r CHOICE
echo
echo ""

case $CHOICE in
    1)
        NEW_VERSION=$(increment_version major "$CURRENT_VERSION")
        ;;
    2)
        NEW_VERSION=$(increment_version minor "$CURRENT_VERSION")
        ;;
    3)
        NEW_VERSION=$(increment_version patch "$CURRENT_VERSION")
        ;;
    4)
        echo "Enter the new version:"
        read -r NEW_VERSION
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

# Validate new version
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$ ]]; then
    echo -e "${RED}Error: Invalid version format${NC}"
    exit 1
fi

echo -e "New version will be: ${GREEN}${NEW_VERSION}${NC}"
echo ""

# Ask what to update
echo "What would you like to update?"
echo "1. Core (version.txt and Cargo.toml)"
echo "2. LangChain integration"
echo "3. LlamaIndex integration"
echo "4. TypeScript SDK"
echo "5. All of the above"
echo ""
read -p "Enter your choice (1-5): " -n 1 -r UPDATE_CHOICE
echo
echo ""

UPDATE_CORE=false
UPDATE_LANGCHAIN=false
UPDATE_LLAMAINDEX=false
UPDATE_TYPESCRIPT=false

case $UPDATE_CHOICE in
    1)
        UPDATE_CORE=true
        ;;
    2)
        UPDATE_LANGCHAIN=true
        ;;
    3)
        UPDATE_LLAMAINDEX=true
        ;;
    4)
        UPDATE_TYPESCRIPT=true
        ;;
    5)
        UPDATE_CORE=true
        UPDATE_LANGCHAIN=true
        UPDATE_LLAMAINDEX=true
        UPDATE_TYPESCRIPT=true
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

# Confirmation
echo -e "${YELLOW}You are about to update:${NC}"
[ "$UPDATE_CORE" = true ] && echo "  - Core (version.txt, Cargo.toml)"
[ "$UPDATE_LANGCHAIN" = true ] && echo "  - LangChain integration (pyproject.toml)"
[ "$UPDATE_LLAMAINDEX" = true ] && echo "  - LlamaIndex integration (pyproject.toml)"
[ "$UPDATE_TYPESCRIPT" = true ] && echo "  - TypeScript SDK (package.json)"
echo ""
echo -e "To version: ${GREEN}${NEW_VERSION}${NC}"
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

echo ""

# Update core
if [ "$UPDATE_CORE" = true ]; then
    echo -e "${BLUE}Updating core version...${NC}"
    
    # Update version.txt
    echo "$NEW_VERSION" > "$PROJECT_ROOT/version.txt"
    echo -e "  ${GREEN}✓${NC} version.txt"
    
    # Update Cargo.toml
    if [ -f "$PROJECT_ROOT/Cargo.toml" ]; then
        sed -i.bak "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" "$PROJECT_ROOT/Cargo.toml"
        rm "$PROJECT_ROOT/Cargo.toml.bak"
        echo -e "  ${GREEN}✓${NC} Cargo.toml"
    fi
fi

# Update LangChain
if [ "$UPDATE_LANGCHAIN" = true ]; then
    echo -e "${BLUE}Updating LangChain integration...${NC}"
    
    if command -v poetry &> /dev/null; then
        cd "$PROJECT_ROOT/integrations/langchain-avocadodb"
        poetry version "$NEW_VERSION" > /dev/null 2>&1
        echo -e "  ${GREEN}✓${NC} langchain-avocadodb/pyproject.toml"
        cd "$PROJECT_ROOT"
    else
        echo -e "  ${YELLOW}⚠${NC} Poetry not found, skipping"
    fi
fi

# Update LlamaIndex
if [ "$UPDATE_LLAMAINDEX" = true ]; then
    echo -e "${BLUE}Updating LlamaIndex integration...${NC}"
    
    if command -v poetry &> /dev/null; then
        cd "$PROJECT_ROOT/integrations/llama-index-avocadodb"
        poetry version "$NEW_VERSION" > /dev/null 2>&1
        echo -e "  ${GREEN}✓${NC} llama-index-avocadodb/pyproject.toml"
        cd "$PROJECT_ROOT"
    else
        echo -e "  ${YELLOW}⚠${NC} Poetry not found, skipping"
    fi
fi

# Update TypeScript SDK
if [ "$UPDATE_TYPESCRIPT" = true ]; then
    echo -e "${BLUE}Updating TypeScript SDK...${NC}"
    
    if command -v npm &> /dev/null; then
        cd "$PROJECT_ROOT/sdks/typescript"
        npm version "$NEW_VERSION" --no-git-tag-version > /dev/null 2>&1
        echo -e "  ${GREEN}✓${NC} typescript/package.json"
        cd "$PROJECT_ROOT"
    else
        echo -e "  ${YELLOW}⚠${NC} npm not found, skipping"
    fi
fi

# Summary
echo ""
echo -e "${GREEN}Version bump complete!${NC}"
echo ""
echo -e "Updated to: ${GREEN}${NEW_VERSION}${NC}"
echo ""
echo "Next steps:"
echo "1. Review the changes: git diff"
echo "2. Run tests to ensure everything works"
echo "3. Commit the changes: git add -A && git commit -m 'chore: Bump version to ${NEW_VERSION}'"
echo "4. Create a release: ./scripts/release.sh"
echo ""
