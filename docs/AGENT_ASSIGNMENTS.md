# Agent Assignments for Radarr MVP

Generated: 2025-08-17 23:03:42

## Complete Agent Registry

## Development Agents

### rust-specialist
- **Description**: Rust development expert
- **Default Model**: sonnet-4
- **Primary Tasks**:
  - Implementation
  - Async patterns
  - Memory management
- **Location**: `~/.claude/agents/categories/development/rust-specialist.md`

### backend-developer
- **Description**: API and service development
- **Default Model**: sonnet-4
- **Primary Tasks**:
  - REST APIs
  - Service layer
  - Middleware
- **Location**: `~/.claude/agents/categories/development/backend-developer.md`

### database-architect
- **Description**: Database design expert
- **Default Model**: opus-4.1
- **Primary Tasks**:
  - Schema design
  - Migrations
  - Query optimization
- **Location**: `~/.claude/agents/categories/development/database-architect.md`

### api-designer
- **Description**: API architecture specialist
- **Default Model**: opus-4.1
- **Primary Tasks**:
  - OpenAPI spec
  - REST design
  - GraphQL
- **Location**: `~/.claude/agents/categories/development/api-designer.md`

## Quality Agents

### test-engineer
- **Description**: Testing specialist
- **Default Model**: sonnet-4
- **Primary Tasks**:
  - Unit tests
  - Integration tests
  - Property tests
- **Location**: `~/.claude/agents/categories/quality/test-engineer.md`

### code-reviewer
- **Description**: Code quality expert
- **Default Model**: sonnet-4
- **Primary Tasks**:
  - Code review
  - Best practices
  - Refactoring
- **Location**: `~/.claude/agents/categories/quality/code-reviewer.md`

### performance-engineer
- **Description**: Performance optimization
- **Default Model**: opus-4.1
- **Primary Tasks**:
  - Benchmarking
  - Profiling
  - Optimization
- **Location**: `~/.claude/agents/categories/quality/performance-engineer.md`

## Infrastructure Agents

### devops-engineer
- **Description**: DevOps and deployment
- **Default Model**: sonnet-4
- **Primary Tasks**:
  - CI/CD
  - Docker
  - Kubernetes
- **Location**: `~/.claude/agents/categories/infrastructure/devops-engineer.md`

### security-auditor
- **Description**: Security specialist
- **Default Model**: opus-4.1
- **Primary Tasks**:
  - Security review
  - Threat modeling
  - Hardening
- **Location**: `~/.claude/agents/categories/infrastructure/security-auditor.md`

## Coordination Agents

### orchestrator
- **Description**: Multi-agent coordinator
- **Default Model**: opus-4.1
- **Primary Tasks**:
  - Planning
  - Coordination
  - Architecture
- **Location**: `~/.claude/agents/categories/coordination/orchestrator.md`

### context-manager
- **Description**: Context preservation
- **Default Model**: sonnet-4
- **Primary Tasks**:
  - Code analysis
  - Pattern detection
  - Documentation
- **Location**: `~/.claude/agents/categories/coordination/context-manager.md`

## Custom Agents

### parser-expert
- **Description**: Release parsing specialist
- **Default Model**: opus-4.1
- **Primary Tasks**:
  - Parser design
  - Regex patterns
  - Scene conventions
- **Location**: `~/.claude/agents/categories/custom/parser-expert.md`

### decision-expert
- **Description**: Decision algorithm specialist
- **Default Model**: opus-4.1
- **Primary Tasks**:
  - Scoring algorithms
  - Quality profiles
  - TRaSH Guides
- **Location**: `~/.claude/agents/categories/custom/decision-expert.md`

### import-specialist
- **Description**: File import specialist
- **Default Model**: sonnet-4
- **Primary Tasks**:
  - Hardlinks
  - File operations
  - Naming templates
- **Location**: `~/.claude/agents/categories/custom/import-specialist.md`

### indexer-specialist
- **Description**: Indexer integration expert
- **Default Model**: sonnet-4
- **Primary Tasks**:
  - API integration
  - Rate limiting
  - RSS feeds
- **Location**: `~/.claude/agents/categories/custom/indexer-specialist.md`


## Feature-to-Agent Mapping

| Feature | Coordinator | Primary Agents | Support Agents | Models |
|---------|-------------|----------------|----------------|--------|
| Movie Management | orchestrator | rust-specialist, backend-developer | database-architect, test-engineer | Opus→Sonnet |
| Release Parser | parser-expert | rust-specialist | test-engineer, performance-engineer | Opus→Sonnet |
| Decision Engine | decision-expert | rust-specialist | performance-engineer | Opus→Sonnet |
| Indexer Integration | indexer-specialist | backend-developer | test-engineer | Opus→Sonnet |
| Import Pipeline | import-specialist | rust-specialist | devops-engineer | Opus→Sonnet |

## Agent Communication Protocol

```json
{
  "from": "orchestrator",
  "to": "rust-specialist",
  "phase": "implementation",
  "model": "sonnet-4",
  "context": "Previous design from database-architect",
  "task": "Implement Movie repository"
}
```

## Workflow Patterns

### Sequential Pattern
```
orchestrator (Opus) → database-architect (Opus) → rust-specialist (Sonnet) → test-engineer (Sonnet)
```

### Parallel Pattern
```
orchestrator (Opus) → ├─ database-architect (Opus)
                      ├─ api-designer (Opus)
                      └─ security-auditor (Opus)
```

## Best Practices

1. **Always start with orchestrator** for complex features
2. **Use specialist agents** for their domains
3. **Follow model guidelines** strictly
4. **Document agent decisions** in code comments
5. **Review with appropriate agents** before merge
