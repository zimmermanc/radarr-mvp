#!/bin/bash

# Complete Setup Script for Radarr MVP with Both Repositories
# Project Path: /home/thetu/radarr-mvp

set -e  # Exit on error

PROJECT_DIR="/home/thetu/radarr-mvp"
CLAUDE_HOME="$HOME/.claude"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_error() { echo -e "${RED}❌ $1${NC}"; }
print_info() { echo -e "${YELLOW}ℹ️  $1${NC}"; }
print_step() { echo -e "${BLUE}📦 $1${NC}"; }

echo "============================================"
echo "Radarr MVP - Complete Setup"
echo "Project: $PROJECT_DIR"
echo "============================================"
echo ""

# Navigate to project directory
cd "$PROJECT_DIR"

# Step 1: Clone Claude Code Studio
print_step "Step 1: Setting up Claude Code Studio"
if [ ! -d "claude-code-studio" ]; then
    print_info "Cloning Claude Code Studio..."
    git clone https://github.com/arnaldo-delisio/claude-code-studio.git
    print_success "Claude Code Studio cloned"
else
    print_success "Claude Code Studio already exists"
    cd claude-code-studio && git pull && cd ..
fi

# Analyze Claude Code Studio structure
print_info "Claude Code Studio structure:"
ls -la claude-code-studio/ | head -10

# Step 2: Clone Awesome Claude Code Subagents
print_step "Step 2: Setting up Awesome Claude Code Subagents"
if [ ! -d "awesome-claude-code-subagents" ]; then
    print_info "Cloning Awesome Subagents..."
    git clone https://github.com/VoltAgent/awesome-claude-code-subagents.git
    print_success "Awesome Subagents cloned"
else
    print_success "Awesome Subagents already exists"
    cd awesome-claude-code-subagents && git pull && cd ..
fi

# Analyze Awesome Subagents structure
print_info "Awesome Subagents structure:"
ls -la awesome-claude-code-subagents/categories/ | head -10

# Step 3: Create Claude home directory structure
print_step "Step 3: Creating Claude home directory structure"
mkdir -p "$CLAUDE_HOME/agents"
mkdir -p "$CLAUDE_HOME/config"
mkdir -p "$CLAUDE_HOME/templates"

# Step 4: Create symlinks for agents
print_step "Step 4: Setting up agent symlinks"

# Remove old symlinks if they exist
rm -f "$CLAUDE_HOME/agents/awesome"
rm -f "$CLAUDE_HOME/agents/categories"

# Link the categories directory (where agents actually are)
ln -sf "$PROJECT_DIR/awesome-claude-code-subagents/categories" "$CLAUDE_HOME/agents/categories"
print_success "Linked categories to ~/.claude/agents/categories"

# Also create individual category links for easier access
mkdir -p "$CLAUDE_HOME/agents/by-category"
for category in "$PROJECT_DIR/awesome-claude-code-subagents/categories"/*; do
    if [ -d "$category" ]; then
        category_name=$(basename "$category")
        ln -sf "$category" "$CLAUDE_HOME/agents/by-category/$category_name"
        echo "  - Linked category: $category_name"
    fi
done

# Step 5: Copy Claude Code Studio templates
print_step "Step 5: Setting up Claude Code Studio templates"
if [ -d "claude-code-studio/templates" ]; then
    cp -r claude-code-studio/templates/* "$CLAUDE_HOME/templates/" 2>/dev/null || true
    print_success "Templates copied"
fi

# Step 6: Create project-specific directories
print_step "Step 6: Creating project structure"
mkdir -p "$PROJECT_DIR/.claude"
mkdir -p "$PROJECT_DIR/.agents/custom"
mkdir -p "$PROJECT_DIR/workflows"
mkdir -p "$PROJECT_DIR/docs"
mkdir -p "$PROJECT_DIR/features"

# Step 7: Create custom agents for Radarr
print_step "Step 7: Creating custom Radarr agents"
cat > "$PROJECT_DIR/.agents/custom/parser-expert.md" << 'EOF'
---
name: parser-expert
description: Media release name parsing specialist
category: custom/media
---

You are a parser-expert specializing in media release name parsing.

## Expertise
- Scene naming conventions
- Quality detection (720p, 1080p, 2160p, etc.)
- Source types (BluRay, WEB-DL, HDTV)
- Audio formats (DTS, TrueHD, AAC)
- Release groups
- Special flags (PROPER, REPACK)

## Focus
- Regex patterns and parser combinators
- Performance: <1ms per parse
- 95%+ accuracy on scene releases
EOF

cat > "$PROJECT_DIR/.agents/custom/decision-expert.md" << 'EOF'
---
name: decision-expert
description: Decision algorithm specialist
category: custom/media
---

You are a decision-expert for automated media selection.

## Expertise
- Multi-factor scoring systems
- Quality profiles
- Custom formats
- TRaSH Guides compatibility
- Upgrade logic

## Focus
- Transparent decisions
- User configurability
- <100ms for 100+ releases
EOF

cat > "$PROJECT_DIR/.agents/custom/import-specialist.md" << 'EOF'
---
name: import-specialist
description: File import pipeline specialist
category: custom/media
---

You are an import-specialist for file operations.

## Expertise
- Hardlink/copy/move strategies
- Cross-filesystem operations
- Naming templates
- Atomic operations
- Error recovery

## Focus
- Data safety first
- <5s per import
- 99%+ success rate
EOF

cat > "$PROJECT_DIR/.agents/custom/indexer-specialist.md" << 'EOF'
---
name: indexer-specialist
description: Indexer integration specialist
category: custom/media
---

You are an indexer-specialist for API integration.

## Expertise
- Prowlarr/Jackett APIs
- Rate limiting
- RSS feeds
- Search orchestration
- Result aggregation

## Focus
- Resilient integration
- <30s for 50 indexers
- 100% rate limit compliance
EOF

print_success "Custom agents created"

# Link custom agents to Claude home
ln -sf "$PROJECT_DIR/.agents/custom" "$CLAUDE_HOME/agents/custom"

# Step 8: Create configuration files
print_step "Step 8: Creating configuration files"

# Create main Claude configuration
cat > "$PROJECT_DIR/.claude/config.json" << EOF
{
  "project": "radarr-mvp",
  "version": "1.0",
  "agents": {
    "enabled": true,
    "sources": [
      "$CLAUDE_HOME/agents/categories",
      "$CLAUDE_HOME/agents/custom"
    ],
    "categories": [
      "development",
      "quality",
      "infrastructure",
      "data",
      "coordination",
      "custom"
    ]
  },
  "workflows": {
    "enabled": true,
    "path": "$PROJECT_DIR/workflows"
  }
}
EOF

# Create agent registry
cat > "$PROJECT_DIR/.agents/registry.json" << EOF
{
  "version": "1.0",
  "standard_agents": {
    "source": "awesome-claude-code-subagents",
    "path": "$PROJECT_DIR/awesome-claude-code-subagents/categories",
    "categories": {
      "development": ["rust-specialist", "backend-developer", "frontend-developer", "api-designer", "database-architect"],
      "quality": ["test-engineer", "code-reviewer", "performance-engineer"],
      "infrastructure": ["devops-engineer", "security-auditor", "docker-specialist"],
      "coordination": ["orchestrator", "context-manager"]
    }
  },
  "custom_agents": {
    "path": "$PROJECT_DIR/.agents/custom",
    "agents": ["parser-expert", "decision-expert", "import-specialist", "indexer-specialist"]
  }
}
EOF

print_success "Configuration files created"

# Step 9: Create sample workflow
print_step "Step 9: Creating sample workflows"

cat > "$PROJECT_DIR/workflows/movie-management.json" << EOF
{
  "name": "movie-management",
  "description": "Complete movie CRUD implementation",
  "coordinator": "orchestrator",
  "phases": [
    {
      "name": "design",
      "agents": ["database-architect", "api-designer"],
      "parallel": true
    },
    {
      "name": "implementation",
      "agents": ["rust-specialist", "backend-developer"],
      "parallel": true
    },
    {
      "name": "testing",
      "agents": ["test-engineer"],
      "parallel": false
    },
    {
      "name": "review",
      "agents": ["code-reviewer", "security-auditor"],
      "parallel": true
    }
  ]
}
EOF

print_success "Workflows created"

# Step 10: Verification
print_step "Step 10: Verifying installation"

echo ""
echo "📋 Checking installations:"
echo "------------------------"

# Check repos
[ -d "$PROJECT_DIR/claude-code-studio" ] && print_success "Claude Code Studio: Installed" || print_error "Claude Code Studio: Missing"
[ -d "$PROJECT_DIR/awesome-claude-code-subagents" ] && print_success "Awesome Subagents: Installed" || print_error "Awesome Subagents: Missing"

# Check symlinks
[ -L "$CLAUDE_HOME/agents/categories" ] && print_success "Agent Categories: Linked" || print_error "Agent Categories: Not linked"
[ -L "$CLAUDE_HOME/agents/custom" ] && print_success "Custom Agents: Linked" || print_error "Custom Agents: Not linked"

# Check config
[ -f "$PROJECT_DIR/.claude/config.json" ] && print_success "Configuration: Created" || print_error "Configuration: Missing"

# Count agents
echo ""
echo "📊 Agent Statistics:"
echo "-------------------"
total_agents=$(find "$PROJECT_DIR/awesome-claude-code-subagents/categories" -name "*.md" | wc -l)
echo "Standard Agents: $total_agents"

custom_agents=$(find "$PROJECT_DIR/.agents/custom" -name "*.md" | wc -l)
echo "Custom Agents: $custom_agents"

echo ""
echo "📁 Agent Categories Available:"
echo "-----------------------------"
for dir in "$PROJECT_DIR/awesome-claude-code-subagents/categories"/*/; do
    category=$(basename "$dir")
    count=$(ls "$dir"/*.md 2>/dev/null | wc -l)
    echo "  - $category: $count agents"
done

echo ""
echo "============================================"
print_success "Setup Complete!"
echo "============================================"
echo ""
echo "Project is ready at: $PROJECT_DIR"
echo ""
echo "Next steps:"
echo "1. cd $PROJECT_DIR"
echo "2. python radarr_project_generator.py"
echo "3. Start development with agents!"
echo ""
echo "Access agents:"
echo "  - Standard: ~/.claude/agents/categories/<category>/<agent>.md"
echo "  - Custom: ~/.claude/agents/custom/<agent>.md"
