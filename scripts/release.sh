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
echo -e "${BLUE}  AvocadoDB Release Script${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# Check if we're on the master branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "master" ]; then
    echo -e "${YELLOW}Warning: You are on branch '${CURRENT_BRANCH}', not 'master'${NC}"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 1
    fi
fi

# Check for uncommitted changes
if [[ -n $(git status -s) ]]; then
    echo -e "${RED}Error: You have uncommitted changes.${NC}"
    git status -s
    echo ""
    echo "Please commit or stash your changes before releasing."
    exit 1
fi

# Get current version
CURRENT_VERSION=$(cat "$PROJECT_ROOT/version.txt" 2>/dev/null || echo "0.0.0")
echo -e "Current version: ${GREEN}${CURRENT_VERSION}${NC}"
echo ""

# Ask for new version
echo "Enter the new version (e.g., 1.0.0, 1.0.0-beta.1):"
read -r NEW_VERSION

# Validate version format (basic semver)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$ ]]; then
    echo -e "${RED}Error: Invalid version format. Use semver (e.g., 1.0.0 or 1.0.0-beta.1)${NC}"
    exit 1
fi

echo ""
echo -e "New version will be: ${GREEN}v${NEW_VERSION}${NC}"
echo ""

# Pre-release checklist
echo -e "${YELLOW}Pre-release Checklist:${NC}"
echo "1. All tests passing?"
echo "2. Documentation updated?"
echo "3. CHANGELOG.md updated?"
echo "4. Breaking changes documented?"
echo "5. Ready to release?"
echo ""
read -p "Have you completed all checklist items? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Please complete the checklist before releasing."
    exit 1
fi

echo ""
echo -e "${BLUE}Starting release process...${NC}"
echo ""

# Step 1: Update version.txt
echo -e "${BLUE}[1/7]${NC} Updating version.txt..."
echo "$NEW_VERSION" > "$PROJECT_ROOT/version.txt"
echo -e "${GREEN}Done${NC}"

# Step 2: Update Cargo.toml
echo -e "${BLUE}[2/7]${NC} Updating Cargo workspace version..."
sed -i.bak "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" "$PROJECT_ROOT/Cargo.toml"
rm "$PROJECT_ROOT/Cargo.toml.bak"
echo -e "${GREEN}Done${NC}"

# Step 3: Generate/update CHANGELOG
echo -e "${BLUE}[3/7]${NC} Checking CHANGELOG.md..."
if [ ! -f "$PROJECT_ROOT/CHANGELOG.md" ]; then
    echo -e "${YELLOW}CHANGELOG.md not found. You may want to generate one.${NC}"
    echo "Run: gh workflow run changelog.yml -f version=${NEW_VERSION}"
else
    echo -e "${GREEN}CHANGELOG.md exists${NC}"
fi

# Step 4: Commit version changes
echo -e "${BLUE}[4/7]${NC} Committing version changes..."
git add "$PROJECT_ROOT/version.txt" "$PROJECT_ROOT/Cargo.toml"
git commit -m "chore: Bump version to ${NEW_VERSION}

Prepare for v${NEW_VERSION} release

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
echo -e "${GREEN}Done${NC}"

# Step 5: Create and push tag
echo -e "${BLUE}[5/7]${NC} Creating git tag v${NEW_VERSION}..."
git tag -a "v${NEW_VERSION}" -m "Release v${NEW_VERSION}

AvocadoDB v${NEW_VERSION}

See CHANGELOG.md for details.

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
echo -e "${GREEN}Done${NC}"

# Step 6: Push to remote
echo -e "${BLUE}[6/7]${NC} Pushing to remote..."
echo ""
echo "This will push the commit and tag to the remote repository."
echo "The tag push will trigger the release workflow."
echo ""
read -p "Push to remote now? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git push origin "$CURRENT_BRANCH"
    git push origin "v${NEW_VERSION}"
    echo -e "${GREEN}Done${NC}"
else
    echo -e "${YELLOW}Skipped. Push manually with:${NC}"
    echo "  git push origin $CURRENT_BRANCH"
    echo "  git push origin v${NEW_VERSION}"
fi

# Step 7: Summary
echo ""
echo -e "${BLUE}[7/7]${NC} Release Summary"
echo -e "${BLUE}======================================${NC}"
echo -e "Version: ${GREEN}v${NEW_VERSION}${NC}"
echo -e "Tag: ${GREEN}v${NEW_VERSION}${NC}"
echo -e "Branch: ${GREEN}${CURRENT_BRANCH}${NC}"
echo ""
echo -e "${GREEN}Release initiated successfully!${NC}"
echo ""
echo "Next steps:"
echo "1. Monitor the release workflow: https://github.com/avocadodb/avocadodb/actions"
echo "2. Wait for binary builds to complete"
echo "3. Verify the GitHub release was created"
echo "4. Test the release binaries"
echo "5. Announce the release"
echo ""
echo -e "${YELLOW}To release integrations, run:${NC}"
echo "  ./scripts/release-integrations.sh ${NEW_VERSION}"
echo ""
