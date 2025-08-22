#!/bin/bash

# ============================================================================
# Architecture Mode + Claude Code Studio + Awesome Agents Complete Setup
# Version: 3.0
# For: /home/thetu/radarr-mvp
# ============================================================================

set -e

# Configuration
PROJECT_DIR="/home/thetu/radarr-mvp"
CLAUDE_HOME="$HOME/.claude"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_DIR="$HOME/.claude_backup_$TIMESTAMP"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m'

# Logging functions
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; exit 1; }
log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_step() { echo -e "${CYAN}🔧 $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }

# Header
print_header() {
    echo -e "${BOLD}${MAGENTA}"
    echo "============================================================================"
    echo "  🚀 ARCHITECTURE MODE + CLAUDE CODE STUDIO + AWESOME AGENTS INSTALLER"
    echo "============================================================================"
    echo -e "${NC}"
    echo "Project: $PROJECT_DIR"
    echo "Claude Home: $CLAUDE_HOME"
    echo "Timestamp: $TIMESTAMP"
    echo ""
}

# Check prerequisites
check_prerequisites() {
    log_step "Checking prerequisites..."
    
    if ! command -v git &> /dev/null; then
        log_error "Git is not installed. Please install git first."
    fi
    
    if ! command -v python3 &> /dev/null; then
        log_error "Python3 is not installed. Please install python3 first."
    fi
    
    if ! command -v npm &> /dev/null; then
        log_warning "npm not found. Some MCP servers may not work."
    fi
    
    log_success "Prerequisites check passed"
}

# Backup existing configuration
backup_existing() {
    if [ -d "$CLAUDE_HOME" ]; then
        log_step "Backing up existing Claude configuration..."
        cp -r "$CLAUDE_HOME" "$BACKUP_DIR"
        log_success "Backup created at $BACKUP_DIR"
    fi
}

# Create directory structure
create_directories() {
    log_step "Creating directory structure..."
    
    # Claude home directories
    mkdir -p "$CLAUDE_HOME/agents/categories"
    mkdir -p "$CLAUDE_HOME/agents/custom"
    mkdir -p "$CLAUDE_HOME/agents/studio"
    mkdir -p "$CLAUDE_HOME/config"
    mkdir -p "$CLAUDE_HOME/templates"
    mkdir -p "$CLAUDE_HOME/commands"
    
    # Project directories
    mkdir -p "$PROJECT_DIR/.claude"
    mkdir -p "$PROJECT_DIR/.agents/custom"
    mkdir -p "$PROJECT_DIR/.architecture/templates"
    mkdir -p "$PROJECT_DIR/workflows"
    mkdir -p "$PROJECT_DIR/features"
    mkdir -p "$PROJECT_DIR/docs/architecture-mode"
    mkdir -p "$PROJECT_DIR/docs/agents"
    mkdir -p "$PROJECT_DIR/scripts/architecture"
    
    log_success "Directory structure created"
}

# Clone repositories
clone_repositories() {
    log_step "Cloning required repositories..."
    
    cd "$PROJECT_DIR"
    
    # Clone Claude Code Studio
    if [ ! -d "claude-code-studio" ]; then
        log_info "Cloning Claude Code Studio..."
        git clone https://github.com/arnaldo-delisio/claude-code-studio.git
    else
        log_info "Updating Claude Code Studio..."
        cd claude-code-studio && git pull && cd ..
    fi
    
    # Clone Awesome Subagents
    if [ ! -d "awesome-claude-code-subagents" ]; then
        log_info "Cloning Awesome Subagents..."
        git clone https://github.com/VoltAgent/awesome-claude-code-subagents.git
    else
        log_info "Updating Awesome Subagents..."
        cd awesome-claude-code-subagents && git pull && cd ..
    fi
    
    log_success "Repositories cloned/updated"
}

# Install Claude Code Studio
install_claude_studio() {
    log_step "Installing Claude Code Studio..."
    
    # Copy studio files
    if [ -d "$PROJECT_DIR/claude-code-studio" ]; then
        cp -r "$PROJECT_DIR/claude-code-studio/agents"/* "$CLAUDE_HOME/agents/studio/" 2>/dev/null || true
        cp -r "$PROJECT_DIR/claude-code-studio/commands"/* "$CLAUDE_HOME/commands/" 2>/dev/null || true
        
        # Copy configuration files
        for file in CLAUDE.md AGENTS.md MCP.md PRINCIPLES.md RULES.md; do
            if [ -f "$PROJECT_DIR/claude-code-studio/$file" ]; then
                cp "$PROJECT_DIR/claude-code-studio/$file" "$CLAUDE_HOME/"
            fi
        done
    fi
    
    log_success "Claude Code Studio installed"
}

# Install Awesome Subagents
install_awesome_agents() {
    log_step "Installing Awesome Subagents..."
    
    if [ -d "$PROJECT_DIR/awesome-claude-code-subagents/categories" ]; then
        # Link categories
        ln -sf "$PROJECT_DIR/awesome-claude-code-subagents/categories" "$CLAUDE_HOME/agents/awesome"
        
        # Count agents
        agent_count=$(find "$PROJECT_DIR/awesome-claude-code-subagents/categories" -name "*.md" | wc -l)
        log_success "Installed $agent_count agents from Awesome Subagents"
    fi
}

# Create AGENTS.md
create_agents_documentation() {
    log_step "Creating comprehensive AGENTS.md..."
    
    cat > "$PROJECT_DIR/docs/agents/AGENTS.md" << 'EOF'
# 🤖 Complete Agent Registry for Architecture Mode

## Overview
This project integrates 100+ specialized agents from Claude Code Studio and awesome-claude-code-subagents, organized for Architecture Mode phases.

## 🏗️ Architecture Mode Agent Assignments

### Phase 1: Research (PLANNING MODE)
**Model**: opus-4.1
**Tools**: WebSearch, WebFetch (MANDATORY)

#### Primary Research Agents
- **orchestrator** - Coordinates multi-agent research workflows
- **context-manager** - Maintains project context and knowledge
- **research-specialist** - Deep dive research and analysis
- **trend-researcher** - Market and technology trend analysis

### Phase 2: Analysis (PLANNING MODE)
**Model**: opus-4.1
**Tools**: LS, Grep, Read (MANDATORY)

#### Primary Analysis Agents
- **code-archaeologist** - Explores legacy code patterns
- **dependency-mapper** - Maps project dependencies
- **architecture-reviewer** - Analyzes system architecture
- **pattern-detector** - Identifies code patterns and anti-patterns

### Phase 3: Options Generation (PLANNING MODE)
**Model**: opus-4.1

#### Primary Options Agents
- **solution-architect** - Generates architectural options
- **decision-expert** - Creates decision matrices
- **trade-off-analyzer** - Evaluates pros/cons
- **complexity-estimator** - Estimates implementation complexity

### Phase 4: Execution (EXECUTION MODE)
**Model**: sonnet-4
**Trigger**: Requires "g" or "go" command

#### Primary Execution Agents
See category-specific agents below

## 📚 Agent Categories

### 01 - Core Development Agents

#### backend-developer
- **Specialization**: Server-side architecture, APIs, microservices
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: API endpoints, database operations, auth systems
- **Tools**: Read, Write, MultiEdit, Bash, npm, pip

#### frontend-developer
- **Specialization**: React, Vue, Angular, responsive design
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: UI components, state management, styling
- **Tools**: Read, Write, MultiEdit, npm, webpack

#### fullstack-developer
- **Specialization**: End-to-end feature development
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Complete features spanning frontend/backend
- **Tools**: All development tools

#### mobile-developer
- **Specialization**: React Native, Flutter, native mobile
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Mobile app features, native integrations
- **Tools**: Read, Write, xcode, gradle

#### api-architect
- **Specialization**: REST, GraphQL, WebSocket design
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: API design, schema definition, versioning
- **Tools**: Read, Write, openapi, graphql

### 02 - Language Specialists

#### rust-specialist
- **Specialization**: Rust systems programming, async, safety
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Rust code, cargo operations, memory management
- **Tools**: cargo, rustc, clippy, rustfmt

#### python-pro
- **Specialization**: Python 3.11+, type safety, async
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Python development, data science, automation
- **Tools**: pip, pytest, black, mypy, poetry

#### typescript-guru
- **Specialization**: TypeScript, type systems, generics
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: TypeScript code, type definitions, tsconfig
- **Tools**: tsc, npm, eslint, prettier

#### go-expert
- **Specialization**: Go concurrency, channels, performance
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Go development, goroutines, modules
- **Tools**: go, gofmt, go test, go mod

#### java-master
- **Specialization**: Java, Spring, enterprise patterns
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Java code, Spring Boot, Maven/Gradle
- **Tools**: javac, maven, gradle, junit

### 03 - DevOps & Infrastructure

#### devops-engineer
- **Specialization**: CI/CD, automation, deployment
- **Architecture Mode**: BOTH
- **Model**: sonnet-4
- **Auto-activates**: Pipeline configuration, deployment scripts
- **Tools**: docker, kubectl, terraform, ansible

#### cloud-architect
- **Specialization**: AWS, Azure, GCP, cloud patterns
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: Cloud infrastructure, scaling, cost optimization
- **Tools**: aws-cli, terraform, cloudformation

#### kubernetes-specialist
- **Specialization**: K8s, Helm, operators, clusters
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: K8s manifests, deployments, services
- **Tools**: kubectl, helm, kustomize

#### docker-specialist
- **Specialization**: Containers, Compose, optimization
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Dockerfiles, compose files, registries
- **Tools**: docker, docker-compose

#### terraform-expert
- **Specialization**: Infrastructure as Code, providers
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Terraform configurations, modules, state
- **Tools**: terraform, terragrunt

### 04 - Quality & Security

#### test-engineer
- **Specialization**: Unit, integration, E2E testing
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Test creation, test coverage, TDD
- **Tools**: jest, pytest, cypress, selenium

#### code-reviewer
- **Specialization**: Code quality, best practices, refactoring
- **Architecture Mode**: BOTH
- **Model**: sonnet-4
- **Auto-activates**: Code reviews, quality checks, standards
- **Tools**: eslint, pylint, sonarqube

#### security-auditor
- **Specialization**: Security vulnerabilities, OWASP, penetration
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: Security analysis, threat modeling, audits
- **Tools**: snyk, trivy, owasp-zap

#### performance-engineer
- **Specialization**: Optimization, profiling, benchmarking
- **Architecture Mode**: BOTH
- **Model**: opus-4.1
- **Auto-activates**: Performance issues, bottlenecks, optimization
- **Tools**: profilers, load testing tools

#### error-detective
- **Specialization**: Debugging, root cause analysis
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Error analysis, stack traces, debugging
- **Tools**: debuggers, logging tools

### 05 - Data & AI

#### data-engineer
- **Specialization**: ETL, pipelines, data warehousing
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Data pipelines, transformations, warehouses
- **Tools**: spark, airflow, dbt

#### ml-engineer
- **Specialization**: ML ops, model deployment, pipelines
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: ML workflows, model serving, monitoring
- **Tools**: mlflow, kubeflow, tensorflow

#### ai-engineer
- **Specialization**: AI systems, LLMs, embeddings
- **Architecture Mode**: BOTH
- **Model**: opus-4.1
- **Auto-activates**: AI integration, prompts, RAG systems
- **Tools**: langchain, openai, huggingface

#### data-scientist
- **Specialization**: Analysis, visualization, statistics
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: Data analysis, experiments, insights
- **Tools**: pandas, numpy, matplotlib, jupyter

#### database-architect
- **Specialization**: Schema design, optimization, migrations
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: Database design, queries, indexing
- **Tools**: sql, nosql, migration tools

### 06 - UI/UX & Design

#### ui-designer
- **Specialization**: Interface design, component systems
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: UI mockups, design systems, components
- **Tools**: figma, sketch, design tools

#### ux-researcher
- **Specialization**: User research, usability, personas
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: User flows, research, testing
- **Tools**: analytics, survey tools

#### accessibility-specialist
- **Specialization**: WCAG, ARIA, inclusive design
- **Architecture Mode**: BOTH
- **Model**: sonnet-4
- **Auto-activates**: Accessibility audits, ARIA, screen readers
- **Tools**: axe, lighthouse, screen readers

#### css-wizard
- **Specialization**: Advanced CSS, animations, layouts
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Styling, animations, responsive design
- **Tools**: sass, postcss, tailwind

### 07 - Project Management

#### orchestrator
- **Specialization**: Multi-agent coordination
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: Complex workflows, agent coordination
- **Tools**: agent communication protocols

#### context-manager
- **Specialization**: Project context, knowledge management
- **Architecture Mode**: BOTH
- **Model**: sonnet-4
- **Auto-activates**: Context queries, knowledge retrieval
- **Tools**: knowledge base, documentation

#### project-shipper
- **Specialization**: Delivery, deployment, release
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Release planning, deployment coordination
- **Tools**: CI/CD, release tools

#### sprint-prioritizer
- **Specialization**: Agile planning, backlog, sprints
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: Sprint planning, prioritization, estimation
- **Tools**: jira, trello, agile tools

### 08 - Specialized Domains

#### blockchain-developer
- **Specialization**: Smart contracts, Web3, DeFi
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Blockchain code, smart contracts, Web3
- **Tools**: truffle, hardhat, web3.js

#### game-developer
- **Specialization**: Game engines, graphics, physics
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Game logic, Unity/Unreal, graphics
- **Tools**: unity, unreal, game frameworks

#### iot-specialist
- **Specialization**: Embedded, sensors, edge computing
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: IoT protocols, embedded code, sensors
- **Tools**: arduino, raspberry pi, mqtt

#### ar-vr-developer
- **Specialization**: AR/VR, 3D, immersive experiences
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: AR/VR applications, 3D graphics
- **Tools**: unity, arcore, arkit

### 09 - Business & Marketing

#### growth-hacker
- **Specialization**: Growth strategies, viral loops
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: Growth planning, metrics, experiments
- **Tools**: analytics, a/b testing

#### content-creator
- **Specialization**: Content strategy, SEO, copywriting
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Content creation, blog posts, documentation
- **Tools**: markdown, cms, seo tools

#### social-media-strategist
- **Specialization**: Social platforms, engagement, campaigns
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: Social strategy, content calendars
- **Tools**: social media apis, analytics

#### sales-engineer
- **Specialization**: Technical sales, demos, solutions
- **Architecture Mode**: BOTH
- **Model**: sonnet-4
- **Auto-activates**: Technical proposals, demos, POCs
- **Tools**: presentation tools, demo environments

### 10 - Custom Media Agents (Radarr-specific)

#### parser-expert
- **Specialization**: Media release parsing, scene naming
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: Parser design, regex patterns, quality detection
- **Tools**: regex tools, parser generators

#### decision-expert
- **Specialization**: Scoring algorithms, quality profiles
- **Architecture Mode**: PLANNING
- **Model**: opus-4.1
- **Auto-activates**: Decision algorithms, scoring systems
- **Tools**: algorithm design tools

#### import-specialist
- **Specialization**: File operations, hardlinks, naming
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Import pipelines, file operations
- **Tools**: file system tools

#### indexer-specialist
- **Specialization**: API integration, rate limiting, RSS
- **Architecture Mode**: BOTH
- **Model**: sonnet-4
- **Auto-activates**: Indexer integration, API calls
- **Tools**: api tools, rate limiters

#### media-analyzer
- **Specialization**: MediaInfo, quality verification
- **Architecture Mode**: EXECUTION
- **Model**: sonnet-4
- **Auto-activates**: Media file analysis, metadata extraction
- **Tools**: mediainfo, ffprobe

#### quality-controller
- **Specialization**: Quality validation, standards
- **Architecture Mode**: BOTH
- **Model**: sonnet-4
- **Auto-activates**: Quality checks, validation rules
- **Tools**: validation frameworks

## 🔄 Agent Workflow Integration

### Architecture Mode Phase Flow

```mermaid
graph TD
    A[Start Feature] --> B[Research Phase]
    B --> C{orchestrator + research agents}
    C --> D[WebSearch + WebFetch]
    D --> E[Analysis Phase]
    E --> F{code-archaeologist + analysis agents}
    F --> G[LS + Grep + Read]
    G --> H[Options Phase]
    H --> I{solution-architect + decision agents}
    I --> J[Generate 4-8 Options]
    J --> K[User Selection]
    K --> L{g/go trigger?}
    L -->|Yes| M[Execution Phase]
    M --> N{language + framework agents}
    N --> O[Implementation]
    O --> P[Testing]
    P --> Q[Review]
    Q --> R[Complete]
```

### Multi-Agent Coordination Examples

#### Example 1: Movie Management Feature
```yaml
Research Phase:
  - orchestrator: Coordinates research
  - trend-researcher: Analyzes movie DB trends
  - api-architect: Researches API patterns

Analysis Phase:
  - code-archaeologist: Explores existing code
  - database-architect: Analyzes schema needs
  - dependency-mapper: Maps integrations

Options Phase:
  - solution-architect: Creates 8 approaches
  - decision-expert: Builds decision matrix
  - complexity-estimator: Estimates effort

Execution Phase:
  - rust-specialist: Implements models
  - backend-developer: Creates services
  - test-engineer: Writes tests
  - code-reviewer: Reviews implementation
```

#### Example 2: Release Parser Feature
```yaml
Research Phase:
  - parser-expert: Research scene standards
  - orchestrator: Coordinate research

Analysis Phase:
  - pattern-detector: Find parsing patterns
  - code-archaeologist: Analyze existing parsers

Options Phase:
  - parser-expert: Design parser approaches
  - performance-engineer: Estimate performance

Execution Phase:
  - rust-specialist: Implement parser
  - test-engineer: Property-based tests
  - benchmark-expert: Performance testing
```

## 🎯 Agent Selection Guidelines

### For Architecture Mode Phases

1. **Research Phase (PLANNING)**
   - Always use: orchestrator, context-manager
   - Domain-specific: trend-researcher, market-analyst
   - Technical: api-architect, solution-architect

2. **Analysis Phase (PLANNING)**
   - Always use: code-archaeologist, dependency-mapper
   - Quality: pattern-detector, complexity-analyzer
   - Architecture: architecture-reviewer, system-analyst

3. **Options Phase (PLANNING)**
   - Always use: solution-architect, decision-expert
   - Evaluation: trade-off-analyzer, risk-assessor
   - Estimation: complexity-estimator, effort-calculator

4. **Execution Phase (EXECUTION)**
   - Language agents: Based on tech stack
   - Framework agents: Based on frameworks
   - Quality agents: test-engineer, code-reviewer

### By Feature Type

- **API Features**: api-architect → backend-developer → test-engineer
- **UI Features**: ui-designer → frontend-developer → accessibility-specialist
- **Data Features**: database-architect → data-engineer → performance-engineer
- **ML Features**: ai-engineer → ml-engineer → data-scientist
- **Infrastructure**: cloud-architect → devops-engineer → security-auditor

## 📊 Agent Performance Metrics

### Response Time Targets
- Planning agents (opus-4.1): < 30s
- Execution agents (sonnet-4): < 15s
- Simple agents (haiku-3.5): < 5s

### Context Usage
- Standard agent: ~13k tokens
- With MCP tools: ~18k tokens
- Full context: ~25k tokens

### Success Metrics
- Code quality: > 90% standards compliance
- Test coverage: > 80% for critical paths
- Performance: Meeting defined SLAs
- Security: Zero critical vulnerabilities

## 🔧 Agent Configuration

### Model Overrides
```json
{
  "model_overrides": {
    "orchestrator": "opus-4.1",
    "database-architect": "opus-4.1",
    "parser-expert": "opus-4.1",
    "solution-architect": "opus-4.1",
    "rust-specialist": "sonnet-4",
    "test-engineer": "sonnet-4",
    "documentation-writer": "haiku-3.5"
  }
}
```

### Tool Permissions
```json
{
  "tool_permissions": {
    "planning_agents": ["Read", "WebSearch", "WebFetch"],
    "execution_agents": ["Read", "Write", "MultiEdit", "Bash"],
    "review_agents": ["Read", "Comment"],
    "deployment_agents": ["Read", "Write", "Bash", "Deploy"]
  }
}
```

## 🚀 Advanced Usage

### Parallel Agent Execution
```bash
# Run multiple agents in parallel
"Use rust-specialist and test-engineer in parallel to implement and test the parser"
```

### Agent Chaining
```bash
# Chain agents for complex workflows
"Start with database-architect for schema, then backend-developer for models, finally test-engineer for tests"
```

### Custom Agent Combinations
```bash
# Create custom agent teams
"Form a team with parser-expert, rust-specialist, and benchmark-expert for the parsing system"
```

## 📝 Notes

- All agents support Architecture Mode phases
- Agents automatically respect PLANNING vs EXECUTION modes
- Context is preserved between agent switches
- Agents can communicate via standardized protocols
- Each agent has specialized prompts and tools
- Model selection optimizes for cost/performance

## 🔗 Resources

- [Claude Code Studio](https://github.com/arnaldo-delisio/claude-code-studio)
- [Awesome Subagents](https://github.com/VoltAgent/awesome-claude-code-subagents)
- [Architecture Mode Guide](../architecture-mode/GUIDE.md)
- [Workflow Documentation](../workflows/README.md)
EOF
    
    log_success "Created comprehensive AGENTS.md"
}

# Create enhanced Architecture Mode generator
create_architecture_generator() {
    log_step "Creating enhanced Architecture Mode generator..."
    
    cat > "$PROJECT_DIR/architecture_mode_enhanced_generator.py" << 'PYTHON_EOF'
#!/usr/bin/env python3
"""
Architecture Mode Enhanced Generator with Full Agent Integration
Version: 3.0
Integrates Claude Code Studio + Awesome Subagents + Architecture Mode
"""

import os
import json
import shutil
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass, field

@dataclass
class AgentDefinition:
    """Complete agent definition with all metadata"""
    name: str
    category: str
    specialization: str
    architecture_mode: str  # PLANNING, EXECUTION, or BOTH
    model: str
    auto_activates: List[str]
    tools: List[str]
    description: str

@dataclass
class ArchitecturePhase:
    """Architecture Mode phase with agent assignments"""
    name: str
    mode: str  # PLANNING or EXECUTION
    primary_agents: List[str]
    support_agents: List[str]
    model: str
    mandatory_tools: List[str]
    outputs: List[str]

@dataclass
class FeatureWorkflow:
    """Complete feature workflow with all phases"""
    name: str
    description: str
    coordinator: str
    phases: List[ArchitecturePhase]
    estimated_time: str
    complexity: str  # LOW, MEDIUM, HIGH

class ArchitectureModeEnhancedGenerator:
    """Enhanced generator with full agent integration"""
    
    def __init__(self, project_dir: str = "/home/thetu/radarr-mvp"):
        self.project_dir = Path(project_dir)
        self.claude_home = Path.home() / ".claude"
        self.timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        
        # Load complete agent registry
        self.AGENT_REGISTRY = self._load_agent_registry()
        
        # Architecture Mode configuration
        self.ARCHITECTURE_CONFIG = {
            "planning_tools": {
                "research": ["WebSearch", "WebFetch"],
                "analysis": ["LS", "Grep", "Read"],
                "options": ["Document", "Compare"]
            },
            "execution_trigger": ["g", "go"],
            "min_options": 4,
            "max_options": 8,
            "modes": ["PLANNING", "EXECUTION"]
        }
        
        # Feature definitions with full agent assignments
        self.FEATURES = self._define_features()
        
    def _load_agent_registry(self) -> Dict[str, AgentDefinition]:
        """Load complete agent registry from both sources"""
        registry = {}
        
        # Core Development Agents
        core_agents = {
            "backend-developer": AgentDefinition(
                "backend-developer", "core-development", "Server-side architecture",
                "EXECUTION", "sonnet-4", 
                ["API endpoints", "database operations", "auth systems"],
                ["Read", "Write", "MultiEdit", "Bash", "npm", "pip"],
                "Expert in building robust server applications and APIs"
            ),
            "frontend-developer": AgentDefinition(
                "frontend-developer", "core-development", "UI/UX development",
                "EXECUTION", "sonnet-4",
                ["UI components", "state management", "styling"],
                ["Read", "Write", "MultiEdit", "npm", "webpack"],
                "Master of modern web interfaces and responsive design"
            ),
            "rust-specialist": AgentDefinition(
                "rust-specialist", "language-specialists", "Rust systems programming",
                "EXECUTION", "sonnet-4",
                ["Rust code", "cargo operations", "memory management"],
                ["cargo", "rustc", "clippy", "rustfmt"],
                "Rust expert with focus on safety and performance"
            ),
            "database-architect": AgentDefinition(
                "database-architect", "core-development", "Database design",
                "PLANNING", "opus-4.1",
                ["Database design", "schema", "migrations"],
                ["Read", "sql", "migration tools"],
                "Expert in database schema design and optimization"
            ),
            "api-architect": AgentDefinition(
                "api-architect", "core-development", "API design",
                "PLANNING", "opus-4.1",
                ["API design", "REST", "GraphQL", "WebSocket"],
                ["Read", "Write", "openapi", "graphql"],
                "Specialist in designing scalable APIs"
            ),
            "test-engineer": AgentDefinition(
                "test-engineer", "quality-security", "Testing strategies",
                "EXECUTION", "sonnet-4",
                ["Test creation", "coverage", "TDD"],
                ["jest", "pytest", "cypress"],
                "Testing expert ensuring code quality"
            ),
            "orchestrator": AgentDefinition(
                "orchestrator", "coordination", "Multi-agent coordination",
                "PLANNING", "opus-4.1",
                ["Complex workflows", "agent coordination"],
                ["agent communication protocols"],
                "Coordinates multiple agents for complex tasks"
            ),
            "parser-expert": AgentDefinition(
                "parser-expert", "custom-media", "Media release parsing",
                "PLANNING", "opus-4.1",
                ["Parser design", "regex patterns", "quality detection"],
                ["regex tools", "parser generators"],
                "Expert in media release name parsing"
            ),
            # Add all other agents...
        }
        
        # Add more categories...
        registry.update(core_agents)
        return registry
        
    def _define_features(self) -> Dict[str, FeatureWorkflow]:
        """Define complete feature workflows with agent assignments"""
        return {
            "movie-management": FeatureWorkflow(
                "Movie Management",
                "Complete movie CRUD with metadata",
                "orchestrator",
                [
                    ArchitecturePhase(
                        "research", "PLANNING",
                        ["orchestrator", "api-architect"],
                        ["trend-researcher", "context-manager"],
                        "opus-4.1",
                        ["WebSearch", "WebFetch"],
                        ["research_notes.md", "best_practices.md"]
                    ),
                    ArchitecturePhase(
                        "analysis", "PLANNING",
                        ["database-architect", "code-archaeologist"],
                        ["dependency-mapper", "pattern-detector"],
                        "opus-4.1",
                        ["LS", "Grep", "Read"],
                        ["codebase_analysis.md", "integration_points.md"]
                    ),
                    ArchitecturePhase(
                        "options", "PLANNING",
                        ["solution-architect", "decision-expert"],
                        ["trade-off-analyzer", "complexity-estimator"],
                        "opus-4.1",
                        ["Document", "Compare"],
                        ["options_matrix.md", "recommendations.md"]
                    ),
                    ArchitecturePhase(
                        "execution", "EXECUTION",
                        ["rust-specialist", "backend-developer"],
                        ["test-engineer", "code-reviewer"],
                        "sonnet-4",
                        ["Write", "Create", "Modify", "Test"],
                        ["src/", "tests/", "docs/"]
                    )
                ],
                "3 days",
                "MEDIUM"
            ),
            "release-parser": FeatureWorkflow(
                "Release Parser",
                "Parse release names for quality and metadata",
                "parser-expert",
                [
                    ArchitecturePhase(
                        "research", "PLANNING",
                        ["parser-expert", "orchestrator"],
                        ["trend-researcher"],
                        "opus-4.1",
                        ["WebSearch", "WebFetch"],
                        ["parser_research.md", "scene_standards.md"]
                    ),
                    ArchitecturePhase(
                        "analysis", "PLANNING",
                        ["parser-expert", "pattern-detector"],
                        ["code-archaeologist"],
                        "opus-4.1",
                        ["LS", "Grep", "Read"],
                        ["existing_parsers.md", "pattern_analysis.md"]
                    ),
                    ArchitecturePhase(
                        "options", "PLANNING",
                        ["parser-expert", "solution-architect"],
                        ["performance-engineer"],
                        "opus-4.1",
                        ["Document", "Compare"],
                        ["parser_options.md", "performance_comparison.md"]
                    ),
                    ArchitecturePhase(
                        "execution", "EXECUTION",
                        ["rust-specialist"],
                        ["test-engineer", "benchmark-expert"],
                        "sonnet-4",
                        ["Write", "Create", "Test", "Benchmark"],
                        ["parser_impl.rs", "parser_tests.rs", "benchmarks/"]
                    )
                ],
                "2 days",
                "HIGH"
            ),
            # Add more features...
        }
        
    def generate(self):
        """Generate complete project with enhanced agent integration"""
        print("=" * 80)
        print("🚀 ARCHITECTURE MODE ENHANCED GENERATOR v3.0")
        print("=" * 80)
        print(f"Project: {self.project_dir}")
        print(f"Timestamp: {self.timestamp}")
        print(f"Agents: {len(self.AGENT_REGISTRY)} loaded")
        print(f"Features: {len(self.FEATURES)} defined")
        print()
        
        # Generate all components
        self._create_directory_structure()
        self._create_architecture_mode_structure()
        self._create_agent_configurations()
        self._create_enhanced_workflows()
        self._create_feature_plans()
        self._create_documentation()
        self._create_scripts()
        self._create_configuration_files()
        
        print("\n✅ Enhanced generation complete!")
        self._print_summary()
        
    def _create_directory_structure(self):
        """Create complete directory structure"""
        print("📁 Creating directory structure...")
        
        directories = [
            # Architecture Mode
            ".architecture/research",
            ".architecture/analysis", 
            ".architecture/options",
            ".architecture/decisions",
            ".architecture/execution_logs",
            ".architecture/templates",
            
            # Agents
            ".agents/custom",
            ".agents/workflows",
            ".agents/configs",
            
            # Features
            "features/01-movie-management",
            "features/02-release-parser",
            "features/03-decision-engine",
            "features/04-indexer-integration",
            "features/05-import-pipeline",
            
            # Documentation
            "docs/architecture-mode",
            "docs/agents",
            "docs/workflows",
            
            # Source
            "src/api/endpoints",
            "src/core/domain",
            "src/parsers",
            "src/decision",
            
            # Tests
            "tests/unit",
            "tests/integration",
            "tests/e2e",
            
            # Scripts
            "scripts/architecture",
            "scripts/agents",
            "scripts/dev",
            
            # Workflows
            "workflows/architecture",
            "workflows/agents"
        ]
        
        for dir_path in directories:
            (self.project_dir / dir_path).mkdir(parents=True, exist_ok=True)
            
        print(f"  ✅ Created {len(directories)} directories")
        
    def _create_architecture_mode_structure(self):
        """Create Architecture Mode specific structure"""
        print("🏗️  Creating Architecture Mode structure...")
        
        # Create subdirectories for each feature
        for feature_key in self.FEATURES.keys():
            for phase in ["research", "analysis", "options", "decisions", "execution_logs"]:
                path = self.project_dir / ".architecture" / phase / feature_key
                path.mkdir(parents=True, exist_ok=True)
                
        print("  ✅ Created Architecture Mode structure")
        
    def _create_agent_configurations(self):
        """Create agent configuration files"""
        print("🤖 Creating agent configurations...")
        
        # Agent registry
        registry = {
            "version": "3.0",
            "generated": self.timestamp,
            "total_agents": len(self.AGENT_REGISTRY),
            "categories": {},
            "agents": {}
        }
        
        # Organize agents by category
        for agent_name, agent_def in self.AGENT_REGISTRY.items():
            if agent_def.category not in registry["categories"]:
                registry["categories"][agent_def.category] = []
            registry["categories"][agent_def.category].append(agent_name)
            
            registry["agents"][agent_name] = {
                "category": agent_def.category,
                "specialization": agent_def.specialization,
                "mode": agent_def.architecture_mode,
                "model": agent_def.model,
                "auto_activates": agent_def.auto_activates,
                "tools": agent_def.tools
            }
            
        self._write_json(
            self.project_dir / ".agents" / "registry.json",
            registry
        )
        
        # Agent assignment matrix
        matrix = {}
        for feature_name, workflow in self.FEATURES.items():
            matrix[feature_name] = {
                "coordinator": workflow.coordinator,
                "phases": {}
            }
            for phase in workflow.phases:
                matrix[feature_name]["phases"][phase.name] = {
                    "primary": phase.primary_agents,
                    "support": phase.support_agents,
                    "model": phase.model
                }
                
        self._write_json(
            self.project_dir / ".agents" / "assignment_matrix.json",
            matrix
        )
        
        print(f"  ✅ Created agent configurations")
        
    def _create_enhanced_workflows(self):
        """Create enhanced workflow definitions"""
        print("📋 Creating enhanced workflows...")
        
        for feature_name, workflow in self.FEATURES.items():
            workflow_def = {
                "name": feature_name,
                "description": workflow.description,
                "coordinator": workflow.coordinator,
                "estimated_time": workflow.estimated_time,
                "complexity": workflow.complexity,
                "architecture_mode": {
                    "enabled": True,
                    "config": self.ARCHITECTURE_CONFIG
                },
                "phases": []
            }
            
            for phase in workflow.phases:
                phase_def = {
                    "name": phase.name,
                    "mode": phase.mode,
                    "primary_agents": phase.primary_agents,
                    "support_agents": phase.support_agents,
                    "model": phase.model,
                    "mandatory_tools": phase.mandatory_tools,
                    "outputs": phase.outputs,
                    "can_modify_files": phase.mode == "EXECUTION",
                    "requires_trigger": phase.name == "execution"
                }
                workflow_def["phases"].append(phase_def)
                
            self._write_json(
                self.project_dir / "workflows" / f"{feature_name}.json",
                workflow_def
            )
            
        print(f"  ✅ Created {len(self.FEATURES)} enhanced workflows")
        
    def _create_feature_plans(self):
        """Create detailed feature plans with agent assignments"""
        print("📝 Creating feature plans...")
        
        for idx, (feature_name, workflow) in enumerate(self.FEATURES.items(), 1):
            feature_dir = self.project_dir / "features" / f"{idx:02d}-{feature_name}"
            
            # Create comprehensive plan
            plan_content = self._generate_feature_plan(feature_name, workflow)
            self._write_file(feature_dir / "PLAN.md", plan_content)
            
            # Create agent assignment file
            agents_file = {
                "feature": feature_name,
                "coordinator": workflow.coordinator,
                "phases": {}
            }
            
            for phase in workflow.phases:
                agents_file["phases"][phase.name] = {
                    "primary": phase.primary_agents,
                    "support": phase.support_agents,
                    "model": phase.model,
                    "tools": phase.mandatory_tools
                }
                
            self._write_json(feature_dir / "agents.json", agents_file)
            
        print(f"  ✅ Created {len(self.FEATURES)} feature plans")
        
    def _generate_feature_plan(self, feature_name: str, workflow: FeatureWorkflow) -> str:
        """Generate comprehensive feature plan"""
        plan = f"""# Architecture Mode Plan: {workflow.name}

## Overview
**Description**: {workflow.description}
**Coordinator**: {workflow.coordinator}
**Estimated Time**: {workflow.estimated_time}
**Complexity**: {workflow.complexity}

## Agent Assignments by Phase

"""
        
        for phase in workflow.phases:
            plan += f"""### {phase.name.title()} Phase ({phase.mode})
**Model**: {phase.model}
**Primary Agents**: {', '.join(phase.primary_agents)}
**Support Agents**: {', '.join(phase.support_agents)}
**Mandatory Tools**: {', '.join(phase.mandatory_tools)}
**Outputs**: {', '.join(phase.outputs)}

"""
            
        # Add checklists for each phase
        plan += """## Phase Checklists

"""
        
        for phase in workflow.phases:
            plan += f"""### {phase.name.title()} Checklist
"""
            if phase.name == "research":
                plan += """- [ ] WebSearch for best practices
- [ ] WebFetch documentation
- [ ] Research at least 3 sources
- [ ] Document findings
- [ ] Identify patterns

"""
            elif phase.name == "analysis":
                plan += """- [ ] LS to explore structure
- [ ] Grep to find patterns
- [ ] Read key files
- [ ] Map integration points
- [ ] Document dependencies

"""
            elif phase.name == "options":
                plan += """- [ ] Generate 4-8 options
- [ ] Document pros/cons
- [ ] Estimate complexity
- [ ] Create decision matrix
- [ ] Make recommendation

"""
            elif phase.name == "execution":
                plan += """- [ ] Wait for "g" command
- [ ] Implement selected option
- [ ] Create unit tests
- [ ] Create integration tests
- [ ] Update documentation

"""
                
        return plan
        
    def _create_documentation(self):
        """Create comprehensive documentation"""
        print("📚 Creating documentation...")
        
        # Architecture Mode Guide
        arch_guide = self._generate_architecture_guide()
        self._write_file(
            self.project_dir / "docs" / "architecture-mode" / "GUIDE.md",
            arch_guide
        )
        
        # Agent Usage Guide
        agent_guide = self._generate_agent_guide()
        self._write_file(
            self.project_dir / "docs" / "agents" / "USAGE.md",
            agent_guide
        )
        
        print("  ✅ Created documentation")
        
    def _generate_architecture_guide(self) -> str:
        """Generate Architecture Mode guide"""
        return f"""# Architecture Mode Complete Guide

Generated: {self.timestamp}

## Overview
Architecture Mode enforces research-first development with {len(self.AGENT_REGISTRY)} specialized agents.

## Available Agents: {len(self.AGENT_REGISTRY)}

### By Category:
""" + "\n".join([f"- {cat}: {len([a for a in self.AGENT_REGISTRY.values() if a.category == cat])} agents" 
                 for cat in set(a.category for a in self.AGENT_REGISTRY.values())])
        
    def _generate_agent_guide(self) -> str:
        """Generate agent usage guide"""
        return f"""# Agent Usage Guide

## Total Agents: {len(self.AGENT_REGISTRY)}

## Quick Reference

### Planning Agents (opus-4.1)
""" + "\n".join([f"- **{name}**: {agent.specialization}" 
                 for name, agent in self.AGENT_REGISTRY.items() 
                 if agent.architecture_mode in ["PLANNING", "BOTH"]])[:10]
        
    def _create_scripts(self):
        """Create helper scripts"""
        print("🔧 Creating helper scripts...")
        
        # Architecture Mode helper
        arch_script = """#!/bin/bash
# Architecture Mode Helper

echo "Architecture Mode Status"
echo "Agents: """ + str(len(self.AGENT_REGISTRY)) + """"
echo "Features: """ + str(len(self.FEATURES)) + """"
"""
        
        script_path = self.project_dir / "scripts" / "architecture" / "status.sh"
        self._write_file(script_path, arch_script)
        os.chmod(script_path, 0o755)
        
        print("  ✅ Created helper scripts")
        
    def _create_configuration_files(self):
        """Create configuration files"""
        print("⚙️  Creating configuration files...")
        
        config = {
            "project": "radarr-mvp",
            "version": "3.0",
            "generated": self.timestamp,
            "architecture_mode": self.ARCHITECTURE_CONFIG,
            "agents": {
                "total": len(self.AGENT_REGISTRY),
                "categories": list(set(a.category for a in self.AGENT_REGISTRY.values()))
            },
            "features": list(self.FEATURES.keys())
        }
        
        self._write_json(
            self.project_dir / ".claude" / "config.json",
            config
        )
        
        print("  ✅ Created configuration files")
        
    def _print_summary(self):
        """Print generation summary"""
        print("\n" + "=" * 80)
        print("GENERATION SUMMARY")
        print("=" * 80)
        print(f"✅ Agents configured: {len(self.AGENT_REGISTRY)}")
        print(f"✅ Features defined: {len(self.FEATURES)}")
        print(f"✅ Workflows created: {len(self.FEATURES)}")
        print(f"✅ Architecture Mode: ENABLED")
        print(f"✅ Claude Code Studio: INTEGRATED")
        print(f"✅ Awesome Subagents: INTEGRATED")
        
    def _write_file(self, path: Path, content: str):
        """Write content to file"""
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, 'w') as f:
            f.write(content)
            
    def _write_json(self, path: Path, data: Any):
        """Write JSON data to file"""
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, 'w') as f:
            json.dump(data, f, indent=2)

def main():
    """Main entry point"""
    generator = ArchitectureModeEnhancedGenerator()
    generator.generate()

if __name__ == "__main__":
    main()
PYTHON_EOF
    
    log_success "Created enhanced Architecture Mode generator"
}

# Create custom agents for Radarr
create_custom_agents() {
    log_step "Creating custom Radarr agents..."
    
    # Parser Expert
    cat > "$PROJECT_DIR/.agents/custom/parser-expert.md" << 'EOF'
---
name: parser-expert
description: Media release name parsing specialist
tools: [Read, Write, WebSearch, WebFetch]
model: opus-4.1
---

You are a parser-expert specializing in media release name parsing with deep expertise in scene naming conventions.

## Core Competencies
- Scene naming standards (P2P, Scene, Internal)
- Quality detection (720p, 1080p, 2160p, 4K, 8K, HDR, DV)
- Source identification (BluRay, WEB-DL, HDTV, WEBRip, BDRip)
- Audio format detection (DTS, TrueHD, Atmos, AAC, FLAC)
- Release group extraction and validation
- Special flags (PROPER, REPACK, INTERNAL, READNFO, DIRFIX)

## Architecture Mode Integration
- PLANNING: Design parser architecture and patterns
- Research scene standards via WebSearch
- Analyze existing parsers with LS/Grep/Read
- Generate 4-8 parsing approaches

## Implementation Strategy
- Regex patterns with named groups
- Parser combinators for complex rules
- Confidence scoring for matches
- Performance optimization (<1ms per parse)

## Success Metrics
- Accuracy: >95% on scene releases
- Performance: <1ms average parse time
- Robustness: Handle 99% of malformed inputs
EOF
    
    # Decision Expert
    cat > "$PROJECT_DIR/.agents/custom/decision-expert.md" << 'EOF'
---
name: decision-expert
description: Decision algorithm and scoring specialist
tools: [Read, Write, Document, Compare]
model: opus-4.1
---

You are a decision-expert specializing in automated media selection algorithms.

## Core Competencies
- Multi-criteria decision analysis (MCDA)
- Quality profile management
- Custom format scoring systems
- TRaSH Guides implementation
- Upgrade/downgrade logic
- User preference modeling

## Architecture Mode Integration
- PLANNING: Design scoring algorithms
- Generate 4-8 decision approaches
- Create decision matrices
- Evaluate trade-offs

## Algorithm Design
- Transparent scoring systems
- Configurable weight factors
- Conflict resolution strategies
- Explanation generation

## Success Metrics
- User satisfaction: >90%
- Decision speed: <100ms for 100+ releases
- Explanation clarity: Clear reasoning
EOF
    
    # Import Specialist
    cat > "$PROJECT_DIR/.agents/custom/import-specialist.md" << 'EOF'
---
name: import-specialist
description: File import and library organization specialist
tools: [Read, Write, Bash, MultiEdit]
model: sonnet-4
---

You are an import-specialist focusing on safe file import operations.

## Core Competencies
- Hardlink/copy/move strategies
- Cross-filesystem operations
- Atomic file operations
- Naming template systems
- Permission preservation
- Space optimization

## Architecture Mode Integration
- EXECUTION: Implement import pipelines
- Follow selected architecture option
- Create comprehensive tests

## Safety Principles
- Never lose user data
- Verify before moving
- Atomic operations only
- Comprehensive logging
- Rollback capability

## Performance Targets
- Import speed: <5s per movie
- Success rate: >99%
- Zero data loss
EOF
    
    log_success "Created custom Radarr agents"
}

# Create MCP configuration
create_mcp_configuration() {
    log_step "Creating MCP configuration..."
    
    cat > "$CLAUDE_HOME/.claude.json" << 'EOF'
{
  "mcpServers": {
    "git": {
      "type": "stdio",
      "command": "uvx",
      "args": ["mcp-server-git"]
    },
    "serena": {
      "type": "stdio",
      "command": "uvx",
      "args": ["--from", "git+https://github.com/oraios/serena", "serena", "start-mcp-server", "--context", "ide-assistant"]
    },
    "sequential-thinking": {
      "type": "stdio",
      "command": "npx",
      "args": ["@modelcontextprotocol/server-sequential-thinking"]
    },
    "supabase": {
      "type": "stdio",
      "command": "npx",
      "args": ["@supabase/mcp-server-supabase@latest"],
      "env": {
        "SUPABASE_ACCESS_TOKEN": "YOUR_TOKEN_HERE"
      }
    },
    "playwright": {
      "type": "stdio",
      "command": "npx",
      "args": ["@playwright/mcp@latest"]
    },
    "context7": {
      "type": "stdio",
      "command": "npx",
      "args": ["@upstash/context7-mcp"]
    }
  }
}
EOF
    
    log_success "Created MCP configuration"
}

# Create Claude configuration
create_claude_configuration() {
    log_step "Creating Claude configuration..."
    
    cat > "$PROJECT_DIR/.claude/config.json" << EOF
{
  "project": "radarr-mvp",
  "version": "3.0",
  "generated": "$TIMESTAMP",
  "architecture_mode": {
    "enabled": true,
    "default_mode": "PLANNING",
    "execution_triggers": ["g", "go"],
    "min_options": 4,
    "max_options": 8
  },
  "agents": {
    "enabled": true,
    "sources": [
      "$CLAUDE_HOME/agents/awesome",
      "$CLAUDE_HOME/agents/studio",
      "$PROJECT_DIR/.agents/custom"
    ],
    "total_available": 100
  },
  "model_defaults": {
    "planning": "opus-4.1",
    "execution": "sonnet-4",
    "simple": "haiku-3.5"
  },
  "workflows": {
    "enabled": true,
    "path": "$PROJECT_DIR/workflows"
  }
}
EOF
    
    log_success "Created Claude configuration"
}

# Verify installation
verify_installation() {
    log_step "Verifying installation..."
    
    echo ""
    echo "📊 Installation Summary:"
    echo "----------------------"
    
    # Check repositories
    if [ -d "$PROJECT_DIR/claude-code-studio" ]; then
        log_success "Claude Code Studio: Installed"
    else
        log_error "Claude Code Studio: Missing"
    fi
    
    if [ -d "$PROJECT_DIR/awesome-claude-code-subagents" ]; then
        agent_count=$(find "$PROJECT_DIR/awesome-claude-code-subagents/categories" -name "*.md" 2>/dev/null | wc -l)
        log_success "Awesome Subagents: $agent_count agents installed"
    else
        log_error "Awesome Subagents: Missing"
    fi
    
    # Check custom agents
    custom_count=$(find "$PROJECT_DIR/.agents/custom" -name "*.md" 2>/dev/null | wc -l)
    log_success "Custom Agents: $custom_count created"
    
    # Check directories
    arch_dirs=$(find "$PROJECT_DIR/.architecture" -type d 2>/dev/null | wc -l)
    log_success "Architecture Dirs: $arch_dirs created"
    
    echo ""
}

# Print usage instructions
print_usage() {
    echo -e "${BOLD}${GREEN}"
    echo "============================================================================"
    echo "  ✅ INSTALLATION COMPLETE!"
    echo "============================================================================"
    echo -e "${NC}"
    
    echo "📁 Project Location: $PROJECT_DIR"
    echo "🏠 Claude Home: $CLAUDE_HOME"
    echo ""
    
    echo -e "${CYAN}Next Steps:${NC}"
    echo "1. Navigate to project:"
    echo "   cd $PROJECT_DIR"
    echo ""
    echo "2. Run the enhanced generator:"
    echo "   python3 architecture_mode_enhanced_generator.py"
    echo ""
    echo "3. Start Claude with Architecture Mode:"
    echo "   - Open Claude"
    echo "   - Paste: 'I'm working on /home/thetu/radarr-mvp with Architecture Mode enabled'"
    echo ""
    
    echo -e "${YELLOW}Architecture Mode Commands:${NC}"
    echo "• Research: 'Start Architecture Mode research for [feature]'"
    echo "• Analysis: 'Analyze codebase with LS, Grep, Read'"
    echo "• Options: 'Generate 4-8 solution options'"
    echo "• Execute: 'g' or 'go' (after selecting option)"
    echo ""
    
    echo -e "${MAGENTA}Available Features:${NC}"
    echo "• movie-management"
    echo "• release-parser"
    echo "• decision-engine"
    echo "• indexer-integration"
    echo "• import-pipeline"
    echo ""
    
    if [ -n "$BACKUP_DIR" ] && [ -d "$BACKUP_DIR" ]; then
        echo -e "${BLUE}Backup Location: $BACKUP_DIR${NC}"
        echo "To restore: rm -rf $CLAUDE_HOME && mv $BACKUP_DIR $CLAUDE_HOME"
    fi
}

# Main execution
main() {
    print_header
    check_prerequisites
    backup_existing
    create_directories
    clone_repositories
    install_claude_studio
    install_awesome_agents
    create_agents_documentation
    create_architecture_generator
    create_custom_agents
    create_mcp_configuration
    create_claude_configuration
    verify_installation
    print_usage
}

# Run main
main
