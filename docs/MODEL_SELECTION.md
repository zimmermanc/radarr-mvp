# Model Selection Guide for Radarr MVP

Generated: 2025-08-17 23:03:42

## Quick Reference

| Model | Use For | Agents |
|-------|---------|--------|
| **Opus 4.1** | Research, Architecture, Complex Algorithms | orchestrator, database-architect, parser-expert |
| **Sonnet 4** | Implementation, APIs, Testing | rust-specialist, backend-developer, test-engineer |
| **Haiku 3.5** | Simple tasks, Config, Formatting | Any agent for simple tasks |

## Detailed Guidelines

### ALWAYS Use Opus 4.1 For:
- Architecture decisions
- Database schema design
- Complex algorithms (parser, decision engine)
- Research phases
- System design
- Performance optimization strategies
- Security architecture

### Use Sonnet 4 For:
- Feature implementation
- API development
- Service code
- Integration tests
- Documentation
- Code refactoring

### Use Haiku 3.5 For:
- Simple CRUD
- Configuration updates
- Code formatting
- README updates
- Simple bug fixes

## Agent Default Models


### Development
- **rust-specialist**: sonnet-4
- **backend-developer**: sonnet-4
- **database-architect**: opus-4.1
- **api-designer**: opus-4.1

### Quality
- **test-engineer**: sonnet-4
- **code-reviewer**: sonnet-4
- **performance-engineer**: opus-4.1

### Infrastructure
- **devops-engineer**: sonnet-4
- **security-auditor**: opus-4.1

### Coordination
- **orchestrator**: opus-4.1
- **context-manager**: sonnet-4

### Custom
- **parser-expert**: opus-4.1
- **decision-expert**: opus-4.1
- **import-specialist**: sonnet-4
- **indexer-specialist**: sonnet-4

## Feature-Specific Model Usage

| Feature | Research | Implementation | Testing |
|---------|----------|----------------|---------|
| Movie Management | Opus 4.1 | Sonnet 4 | Sonnet 4 |
| Release Parser | Opus 4.1 | Sonnet 4 | Sonnet 4 |
| Decision Engine | Opus 4.1 | Sonnet 4 | Sonnet 4 |
| Indexer Integration | Opus 4.1 | Sonnet 4 | Sonnet 4 |
| Import Pipeline | Opus 4.1 | Sonnet 4 | Sonnet 4 |

## Workflow Example

```
1. Research (Opus 4.1 + orchestrator)
   └─> Understand requirements, design architecture

2. Design (Opus 4.1 + specialist)
   └─> Database schema, API design, algorithms

3. Implementation (Sonnet 4 + developers)
   └─> Write code, create services, build APIs

4. Testing (Sonnet 4 + test-engineer)
   └─> Unit tests, integration tests, e2e tests

5. Optimization (Opus 4.1 + performance-engineer)
   └─> Profile, benchmark, optimize

6. Simple fixes (Haiku 3.5)
   └─> Typos, formatting, simple updates
```
