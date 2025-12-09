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
echo -e "${BLUE}  AvocadoDB Integration Release${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

# Get version from argument or prompt
if [ -n "$1" ]; then
    VERSION="$1"
else
    echo "Enter the version to release (e.g., 1.0.0):"
    read -r VERSION
fi

# Validate version format
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$ ]]; then
    echo -e "${RED}Error: Invalid version format. Use semver (e.g., 1.0.0)${NC}"
    exit 1
fi

echo -e "Releasing integrations for version: ${GREEN}${VERSION}${NC}"
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

# Ask which integrations to release
echo "Which integrations would you like to release?"
echo ""
echo "1. LangChain (langchain-avocadodb)"
echo "2. LlamaIndex (llama-index-avocadodb)"
echo "3. TypeScript SDK (avocadodb npm package)"
echo "4. All of the above"
echo ""
read -p "Enter your choice (1-4): " -n 1 -r CHOICE
echo
echo ""

RELEASE_LANGCHAIN=false
RELEASE_LLAMAINDEX=false
RELEASE_TYPESCRIPT=false

case $CHOICE in
    1)
        RELEASE_LANGCHAIN=true
        ;;
    2)
        RELEASE_LLAMAINDEX=true
        ;;
    3)
        RELEASE_TYPESCRIPT=true
        ;;
    4)
        RELEASE_LANGCHAIN=true
        RELEASE_LLAMAINDEX=true
        RELEASE_TYPESCRIPT=true
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

# Confirmation
echo -e "${YELLOW}You are about to release:${NC}"
[ "$RELEASE_LANGCHAIN" = true ] && echo "  - langchain-avocadodb v${VERSION}"
[ "$RELEASE_LLAMAINDEX" = true ] && echo "  - llama-index-avocadodb v${VERSION}"
[ "$RELEASE_TYPESCRIPT" = true ] && echo "  - avocadodb (npm) v${VERSION}"
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

echo ""

# Release LangChain
if [ "$RELEASE_LANGCHAIN" = true ]; then
    echo -e "${BLUE}Releasing langchain-avocadodb...${NC}"
    
    # Update version in pyproject.toml
    cd "$PROJECT_ROOT/integrations/langchain-avocadodb"
    poetry version "$VERSION"
    
    # Commit and tag
    cd "$PROJECT_ROOT"
    git add integrations/langchain-avocadodb/pyproject.toml
    git commit -m "chore(langchain): Bump version to ${VERSION}

Prepare langchain-avocadodb for v${VERSION} release

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
    
    git tag -a "langchain-v${VERSION}" -m "Release langchain-avocadodb v${VERSION}

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
    
    echo -e "${GREEN}Tagged langchain-v${VERSION}${NC}"
    echo ""
fi

# Release LlamaIndex
if [ "$RELEASE_LLAMAINDEX" = true ]; then
    echo -e "${BLUE}Releasing llama-index-avocadodb...${NC}"
    
    # Update version in pyproject.toml
    cd "$PROJECT_ROOT/integrations/llama-index-avocadodb"
    poetry version "$VERSION"
    
    # Commit and tag
    cd "$PROJECT_ROOT"
    git add integrations/llama-index-avocadodb/pyproject.toml
    git commit -m "chore(llamaindex): Bump version to ${VERSION}

Prepare llama-index-avocadodb for v${VERSION} release

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
    
    git tag -a "llamaindex-v${VERSION}" -m "Release llama-index-avocadodb v${VERSION}

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
    
    echo -e "${GREEN}Tagged llamaindex-v${VERSION}${NC}"
    echo ""
fi

# Release TypeScript SDK
if [ "$RELEASE_TYPESCRIPT" = true ]; then
    echo -e "${BLUE}Releasing TypeScript SDK...${NC}"
    
    # Update version in package.json
    cd "$PROJECT_ROOT/sdks/typescript"
    npm version "$VERSION" --no-git-tag-version
    
    # Commit and tag
    cd "$PROJECT_ROOT"
    git add sdks/typescript/package.json
    git commit -m "chore(typescript): Bump version to ${VERSION}

Prepare TypeScript SDK for v${VERSION} release

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
    
    git tag -a "npm-v${VERSION}" -m "Release avocadodb (npm) v${VERSION}

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
    
    echo -e "${GREEN}Tagged npm-v${VERSION}${NC}"
    echo ""
fi

# Push to remote
echo -e "${BLUE}Pushing to remote...${NC}"
echo ""
echo "This will push the commits and tags to the remote repository."
echo "The tag pushes will trigger the respective release workflows."
echo ""
read -p "Push to remote now? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git push origin "$CURRENT_BRANCH"
    
    [ "$RELEASE_LANGCHAIN" = true ] && git push origin "langchain-v${VERSION}"
    [ "$RELEASE_LLAMAINDEX" = true ] && git push origin "llamaindex-v${VERSION}"
    [ "$RELEASE_TYPESCRIPT" = true ] && git push origin "npm-v${VERSION}"
    
    echo -e "${GREEN}Done${NC}"
else
    echo -e "${YELLOW}Skipped. Push manually with:${NC}"
    echo "  git push origin $CURRENT_BRANCH"
    [ "$RELEASE_LANGCHAIN" = true ] && echo "  git push origin langchain-v${VERSION}"
    [ "$RELEASE_LLAMAINDEX" = true ] && echo "  git push origin llamaindex-v${VERSION}"
    [ "$RELEASE_TYPESCRIPT" = true ] && echo "  git push origin npm-v${VERSION}"
fi

# Summary
echo ""
echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}Integration releases initiated!${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""
echo "Monitor the workflows:"
echo "  - GitHub Actions: https://github.com/avocadodb/avocadodb/actions"
echo ""
[ "$RELEASE_LANGCHAIN" = true ] && echo "  - LangChain PyPI: https://pypi.org/project/langchain-avocadodb/"
[ "$RELEASE_LLAMAINDEX" = true ] && echo "  - LlamaIndex PyPI: https://pypi.org/project/llama-index-avocadodb/"
[ "$RELEASE_TYPESCRIPT" = true ] && echo "  - npm: https://www.npmjs.com/package/avocadodb"
echo ""
