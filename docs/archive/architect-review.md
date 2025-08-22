# Architect's Review Process for 01-plan.md Updates

**Purpose**: Systematic review process for architects to assess junior developer work and update project documentation  
**Usage**: Run this when returning to the project after development work has been done  
**Model**: Use Opus 4.1 for architectural assessment

---

## 🎯 Architect's Review Workflow

### Phase 1: Rapid Assessment (5 minutes)

```bash
# 1. Run the quick assessment to get current state
bash /home/thetu/radarr-mvp/assessment_run.md

# 2. Check git history to see what changed
cd /home/thetu/radarr-mvp
git log --oneline -20
git diff HEAD~10 --stat

# 3. Look for progress tracking
cat PROGRESS.md 2>/dev/null || echo "No progress file found"
```

### Phase 2: Deep Technical Assessment (15 minutes)

Use specialized agents for comprehensive analysis:

```yaml
assessment_tasks:
  - agent: architect-reviewer
    prompt: |
      Review the Radarr MVP codebase at /home/thetu/radarr-mvp/unified-radarr
      Focus on:
      1. What has changed since the last assessment (check git history)
      2. Current compilation status and test results
      3. Architectural integrity - are changes following clean architecture?
      4. Technical debt introduced or resolved
      5. Performance implications of changes
      6. Security considerations
      
      Compare against the original plan in 01-plan.md and report:
      - What was supposed to be done (from 03-tasks.md)
      - What was actually done
      - Quality of implementation
      - Remaining work
      - Revised estimates

  - agent: code-reviewer  
    prompt: |
      Review recent code changes in /home/thetu/radarr-mvp/unified-radarr
      Run: git diff HEAD~20 to see changes
      Assess:
      - Code quality and standards adherence
      - Error handling completeness
      - Test coverage for new code
      - Performance considerations
      - Security vulnerabilities

  - agent: test-results-analyzer
    prompt: |
      Analyze test suite results for Radarr MVP
      Run: cd /home/thetu/radarr-mvp/unified-radarr && cargo test --workspace
      Report:
      - Test pass/fail rates by component
      - New tests added vs planned
      - Integration test coverage
      - Performance test results if any
      - Recommendations for missing tests
```

### Phase 3: Comprehensive Status Update (20 minutes)

After gathering assessment data, update 01-plan.md systematically:

---

## 📋 01-plan.md Update Checklist

### 1. Update Header Metadata
```markdown
**Last Updated**: 2025-08-XX (Architect Review - Week X Complete)  
**Current State**: [New state based on assessment]
**Lines of Code**: [Run: find unified-radarr -name "*.rs" | xargs wc -l]
**Completion**: ~X% Overall, Y% Functional (revised from previous)
```

### 2. Create/Update Progress Section
```markdown
## 📈 Progress Since Last Architect Review

### Completed (Week X)
Based on git history and assessment:
- ✅ [Task from 03-tasks.md] - [Actual implementation quality: Good/Fair/Poor]
- ✅ [Another task] - [Quality assessment]
- ⚠️ [Partial task] - [What's done, what's missing]
- ❌ [Failed/Skipped task] - [Why it wasn't completed]

### Deviations from Plan
- **Planned but not done**: [List items from 03-tasks.md not completed]
- **Done but not planned**: [Unexpected work that was needed]
- **Quality issues**: [Subpar implementations needing rework]

### Technical Debt Assessment
- **Added**: [New technical debt introduced]
- **Resolved**: [Technical debt eliminated]
- **Net change**: [Overall debt trajectory]
```

### 3. Update Component Status with Evidence
```markdown
### Actual Component Status (Verified 2025-08-XX)
```
Component                   | Before | After | Change | Evidence
---------------------------|--------|-------|--------|----------
radarr-core                | 90%    | 95%   | +5% ⬆️  | 5 new models added
radarr-infrastructure      | 30%    | 100%  | +70% ⬆️ | All repos working
radarr-api                 | 15%    | 45%   | +30% ⬆️ | 8/20 endpoints done
```
Include verification commands:
- Verified by: `cargo test -p radarr-core` (45/45 passing)
- API tested: `curl localhost:7878/api/v3/movie` (working)
```

### 4. Revise Risk Assessment
```markdown
## Risk Assessment (Architect Review 2025-08-XX)

### Risks Materialized
| Risk | Impact | What Happened | Mitigation |
|------|--------|---------------|------------|
| [Risk that occurred] | [Actual impact] | [Description] | [How to prevent] |

### New Risks Identified
| Risk | Impact | Probability | Evidence |
|------|--------|-------------|----------|
| [New risk from code review] | High | Medium | [Code showing risk] |
```

### 5. Adjust Timeline Based on Velocity
```markdown
## Revised Timeline (Based on Actual Velocity)

### Velocity Analysis
- **Planned**: X tasks/week
- **Actual**: Y tasks/week  
- **Efficiency**: Y/X = Z%

### Updated Schedule
Original: 8 weeks
Current week: X
Remaining work: [List]
Revised completion: Week Y (± variance)

### Critical Path Changes
- [What's now blocking progress]
- [What can be parallelized]
- [What should be deprioritized]
```

### 6. Update Recommendations
```markdown
## Architect's Recommendations

### Immediate Actions (Next Sprint)
1. **Critical**: [Must fix now - blocking issues]
2. **High**: [Important for momentum]
3. **Medium**: [Nice to have if time]

### Architecture Adjustments
- [Design changes needed based on implementation reality]
- [Patterns that aren't working]
- [Simplifications possible]

### Process Improvements
- [What slowed development]
- [Missing tools or setup]
- [Documentation gaps found]
```

---

## 🤖 Automated Assessment Script

Create this as `architect-assessment.sh`:

```bash
#!/bin/bash

echo "=== ARCHITECT'S COMPREHENSIVE ASSESSMENT ==="
echo "Date: $(date)"
echo ""

cd /home/thetu/radarr-mvp/unified-radarr || exit 1

# 1. Code metrics
echo "📊 CODE METRICS:"
echo "Total Rust files: $(find . -name "*.rs" | wc -l)"
echo "Total lines: $(find . -name "*.rs" | xargs wc -l | tail -1)"
echo "TODO comments: $(grep -r "TODO" --include="*.rs" | wc -l)"
echo "FIXME comments: $(grep -r "FIXME" --include="*.rs" | wc -l)"
echo ""

# 2. Git activity
echo "📝 RECENT ACTIVITY:"
echo "Commits in last week: $(git log --since=1.week --oneline | wc -l)"
echo "Files changed: $(git diff --stat HEAD~20 | tail -1)"
echo "Contributors: $(git log --since=1.week --format='%an' | sort -u)"
echo ""

# 3. Compilation and tests
echo "🔨 BUILD STATUS:"
if cargo build --workspace 2>&1 | grep -q "Finished"; then
    echo "✅ Compilation: SUCCESS"
    
    # Run tests
    TEST_RESULT=$(cargo test --workspace 2>&1)
    PASSED=$(echo "$TEST_RESULT" | grep -oP '\d+(?= passed)' | tail -1)
    FAILED=$(echo "$TEST_RESULT" | grep -oP '\d+(?= failed)' | tail -1)
    echo "✅ Tests: $PASSED passed, $FAILED failed"
else
    ERRORS=$(cargo build --workspace 2>&1 | grep -c "error\[")
    echo "❌ Compilation: FAILED ($ERRORS errors)"
fi
echo ""

# 4. Feature completion
echo "✨ FEATURE STATUS:"
for crate in core infrastructure api indexers downloaders import decision; do
    if [ -d "crates/$crate" ]; then
        TEST_COUNT=$(find crates/$crate -name "*.rs" -exec grep -l "#\[test\]" {} \; | wc -l)
        echo "$crate: $TEST_COUNT test files"
    fi
done
echo ""

# 5. Dependencies
echo "📦 DEPENDENCY HEALTH:"
cargo audit 2>/dev/null || echo "cargo-audit not installed"
cargo outdated 2>/dev/null || echo "cargo-outdated not installed"
echo ""

# 6. Performance baseline
echo "⚡ PERFORMANCE CHECK:"
if [ -f "target/release/radarr-mvp" ]; then
    SIZE=$(du -h target/release/radarr-mvp | cut -f1)
    echo "Binary size: $SIZE"
fi

# Simple benchmark if server runs
timeout 2 cargo run --release 2>/dev/null &
sleep 1
if curl -s localhost:7878/health > /dev/null 2>&1; then
    RESPONSE_TIME=$(curl -w "%{time_total}" -o /dev/null -s localhost:7878/health)
    echo "Health check response: ${RESPONSE_TIME}s"
fi
pkill -f "cargo run" 2>/dev/null

echo ""
echo "=== ASSESSMENT COMPLETE ==="
```

---

## 🎯 Quick Decision Framework

Based on assessment results:

### If Compilation Working + Tests Passing
→ Update completion percentage significantly  
→ Focus review on code quality and architecture  
→ Plan next sprint based on velocity  

### If Compilation Working + Tests Failing  
→ Analyze test failures for root cause  
→ Determine if tests or code need fixing  
→ Adjust timeline for debugging  

### If Compilation Still Broken
→ Identify what work was actually done  
→ Determine why plan wasn't followed  
→ Consider intervention or support needed  

### If Ahead of Schedule
→ Document what enabled faster progress  
→ Consider adding stretch goals  
→ Update timeline optimistically  

### If Behind Schedule  
→ Identify blockers and complexity underestimation  
→ Revise scope or timeline  
→ Consider architectural simplifications  

---

## 📝 Template for Architect's Summary

After assessment, add this to 01-plan.md:

```markdown
## 🏗️ Architect Review - Week X

**Review Date**: 2025-08-XX  
**Reviewer**: [Model/Agent used]  
**Development Period**: Week X of 8  

### Executive Summary
[2-3 sentences on overall progress and health]

### Work Completed vs Planned
- **Planned**: X tasks from 03-tasks.md
- **Completed**: Y tasks (Z% plan adherence)  
- **Quality Score**: [A-F grade based on code review]
- **Velocity**: [Ahead/On Track/Behind] schedule

### Key Findings
1. **Success**: [What went well]
2. **Challenge**: [What struggled]  
3. **Surprise**: [Unexpected discovery]

### Revised Projections
- **Original Timeline**: 8 weeks
- **Current Progress**: X%
- **Projected Completion**: Week Y
- **Confidence Level**: [High/Medium/Low]

### Next Sprint Priority
1. [Most critical task]
2. [Second priority]
3. [Third priority]

### Architecture Decision
[Any design changes needed based on implementation reality]
```