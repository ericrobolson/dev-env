# Research: Configurable Agent Environment Variables

## Prior Art & Existing Solutions

### Current Implementation

| File | Path | Purpose |
|------|------|---------|
| helpers.sh | `bin/helpers.sh` | Centralized configuration via `init_globals()` function. Already supports env vars with defaults. |
| build-feature | `bin/build-feature` | Sources helpers.sh, uses `run_agent()` abstraction. |
| gen-doc | `bin/gen-doc` | Sources helpers.sh, uses `run_agent()` abstraction. |
| README.md | `README.md` | Has outdated "Configuration" section with hardcoded claims and TODO placeholder in env vars table. |
| TODOS.md | `TODOS.md` | Tracks this work as incomplete item: "Update bin/build-feature and bin/gen-doc to be agent agnostic/configurable". |

### Related Completed Work

| Feature | Path | Relevance |
|---------|------|-----------|
| Behavior Consolidation | `docs/2603110930-BehaviorConsolidation/` | Created helpers.sh with `init_globals()`, `run_agent()`, `wait_for_user()`, `build_prompt()`. Established pattern for shared configuration. |
| Convert OneShot | `docs/2603110912-ConvertOneShot/` | Migrated gen-doc to use helpers.sh pattern. |

## Current State Analysis

### helpers.sh (line 25-38)

```bash
init_globals() {
    TIME_STAMP=$(date +%y%m%d%H%M)
    IDE="${IDE:-cursor}"              # Default: cursor
    AGENT_TYPE="${AGENT_TYPE:-cursor}"  # TO CHANGE: default "claude"
    CURSOR_MODEL="${CURSOR_MODEL:-kimi-k2.5}"  # Default: kimi-k2.5
    TERSENESS="..."
    
    # Validation exists for AGENT_TYPE (line 33-37)
    if [[ "$AGENT_TYPE" != "cursor" && "$AGENT_TYPE" != "claude" ]]; then
        echo "Error: AGENT_TYPE must be 'cursor' or 'claude', got: $AGENT_TYPE" >&2
        return 1
    fi
}
```

Status: Already implemented. Only needs default change for AGENT_TYPE.

### run_agent() (line 40-65)

```bash
run_agent() {
    local stage="$1"
    prompt=$(cat)
    
    if [[ "$AGENT_TYPE" == "claude" ]]; then
        echo "$prompt" | claude --dangerously-skip-permissions
    else
        echo "$prompt" | cursor-agent --model "$CURSOR_MODEL"
    fi
}
```

Status: Already supports both agents. Uses CURSOR_MODEL for cursor-agent.

### build_prompt() (line 106-122)

References IDE variable for opening files:
```bash
"Then open the file '$filepath' in the IDE '$IDE' so I can review it."
```

Status: Already uses IDE variable. No IDE validation performed (pass-through).

## Intricacies & Challenges

### 1. AGENT_TYPE Default Change Impact

| Scenario | Current (cursor) | New (claude) | Risk |
|----------|-----------------|--------------|------|
| User has no env vars set | Uses cursor-agent | Uses claude CLI | **Breaking** - requires claude CLI installed |
| User has cursor installed only | Works | **Fails** - claude not found | High |
| User has both installed | Works | Works | None |
| User already set AGENT_TYPE=cursor | Works | Works | None |

**Mitigation**: Users depending on cursor must explicitly set `AGENT_TYPE=cursor` after update.

### 2. CLI Availability

| CLI | Installation | Availability Check |
|-----|--------------|-------------------|
| claude | `npm install -g @anthropic-ai/claude-cli` | No runtime check in current code. Command fails with "not found" error. |
| cursor-agent | Bundled with Cursor IDE | Assumed available if AGENT_TYPE=cursor selected. |

**Challenge**: No pre-flight check for claude CLI availability. Error surfaces at runtime during `run_agent()` execution.

### 3. Model Validation

| Approach | Pros | Cons |
|----------|------|------|
| Pass-through (current) | Flexible, supports new models immediately | Invalid models fail silently or with cryptic errors |
| Validate against allowlist | Clear error messages | Requires maintenance when new models released |

Current implementation: Pass-through. cursor-agent receives model string directly.

Known cursor-agent models (from codebase references):
- `kimi-k2.5` (default)
- `opus-4.5-thinking`

### 4. IDE Values

| IDE Value | Expected Behavior | Actual Behavior |
|-----------|------------------|-----------------|
| cursor | Opens in Cursor IDE | Dependent on agent implementation in build_prompt |
| vscode | Opens in VSCode | Pass-through, assumes agent handles it |
| Any string | Passed to agent | No validation |

**Challenge**: IDE value is passed blindly to the agent's prompt. Actual IDE opening depends on agent's ability to interpret and execute.

### 5. Cross-Script Consistency

| Script | Sources helpers.sh | Uses init_globals() | Uses run_agent() |
|--------|-------------------|---------------------|------------------|
| build-feature | Yes (line 17-18) | Yes (line 34) | Yes (line 60, 80, 102, 115, 128) |
| gen-doc | Yes (line 12-13) | Yes (line 28) | Yes (line 42) |

Both scripts properly delegate to helpers.sh. No individual script changes needed for env var support.

### 6. Non-Interactive Behavior

| Variable | Interactive | Non-Interactive (`[[ ! -t 0 ]]`) |
|----------|-------------|----------------------------------|
| wait_for_user | Prompts for y/n | Skips (returns 0) |
| AGENT_TYPE=claude | Skips prompt | Skips prompt |
| AGENT_TYPE=cursor | Prompts for y/n | Skips if non-interactive |

**Challenge**: Changing default to claude removes interactive prompts by default. Users may expect pause behavior that no longer occurs.

### 7. Migration Documentation

README.md sections requiring update:

| Section | Line | Current State | Required Change |
|---------|------|---------------|-----------------|
| Environment Variables table | 24-28 | TODO placeholder | Document IDE, AGENT_TYPE, CURSOR_MODEL |
| Configuration section | 161-179 | Claims hardcoded values | Document env var usage |
| Models table | 174-177 | Hardcoded locations | Update to reflect env var configuration |

## Testing Matrix

| Test ID | Scenario | Expected Result |
|---------|----------|-----------------|
| T1 | No env vars | AGENT_TYPE=claude, IDE=cursor, CURSOR_MODEL=kimi-k2.5 |
| T2 | AGENT_TYPE=claude explicitly | Uses claude CLI |
| T3 | AGENT_TYPE=cursor explicitly | Uses cursor-agent --model |
| T4 | AGENT_TYPE=invalid | Error exit 1 with message |
| T5 | IDE=vscode | Passed through to prompt |
| T6 | CURSOR_MODEL=custom | Passed to cursor-agent --model |
| T7 | Missing claude CLI | Runtime error "command not found" |

## Relevant Files Summary

| File | Lines | Summary |
|------|-------|---------|
| `bin/helpers.sh` | 1-123 | **Core configuration file.** Contains `init_globals()`, `run_agent()`, `wait_for_user()`, `build_prompt()`. Single source of truth for env var handling. |
| `bin/build-feature` | 1-132 | Multi-stage pipeline. Sources helpers.sh. Needs no changes for env var support. |
| `bin/gen-doc` | 1-43 | Single-document generator. Sources helpers.sh. Needs no changes for env var support. |
| `README.md` | 1-192 | **Requires documentation updates.** Lines 24-28 (env vars), 161-179 (config section), 174-177 (models table). |
| `TODOS.md` | 1-12 | Tracks this work. Item 8-11 should be marked complete after implementation. |
| `docs/2603110930-BehaviorConsolidation/03-plan.md` | 1-438 | Reference for helpers.sh design rationale and implementation patterns. |
