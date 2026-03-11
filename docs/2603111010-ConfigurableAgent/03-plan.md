# Implementation Plan: Configurable Agent Environment Variables

References:
- [Design Document](docs/2603111010-ConfigurableAgent/01-design.md)
- [Research Document](docs/2603111010-ConfigurableAgent/02-research.md)

## Files to Modify

### 1. bin/helpers.sh

Change default `AGENT_TYPE` from "cursor" to "claude" (line 30):

```bash
init_globals() {
    TIME_STAMP=$(date +%y%m%d%H%M)
    IDE="${IDE:-cursor}"
    AGENT_TYPE="${AGENT_TYPE:-claude}"  # Changed default
    CURSOR_MODEL="${CURSOR_MODEL:-kimi-k2.5}"
    # ... rest of function
}
```

### 2. README.md

Update Environment Variables table (lines 24-28):

```markdown
| Variable | Default | Description |
|----------|---------|-------------|
| `IDE` | `cursor` | IDE to open files in (cursor, vscode, etc) |
| `AGENT_TYPE` | `claude` | Agent to use: `claude` or `cursor` |
| `CURSOR_MODEL` | `kimi-k2.5` | Model for cursor-agent (e.g., opus-4.5-thinking) |
```

Update Configuration section (lines 161-179):

```markdown
## Configuration

Configure behavior via environment variables:

```bash
# In ~/.zshrc or shell profile
export AGENT_TYPE=claude    # or cursor
export IDE=cursor           # or vscode
export CURSOR_MODEL=kimi-k2.5
```

The `run_agent()` function in `bin/helpers.sh` dispatches to the selected agent.
```

Remove "hardcoded" claims and outdated TODO placeholder.

### 3. TODOS.md

Mark item complete (lines 8-11):

```markdown
- [x] Update bin/build-feature and bin/gen-doc to be agent agnostic/configurable
  - Default AGENT_TYPE changed to "claude"
  - All env vars documented in README.md
```

## Validation Already Implemented

`bin/helpers.sh` already validates `AGENT_TYPE`:

```bash
if [[ "$AGENT_TYPE" != "cursor" && "$AGENT_TYPE" != "claude" ]]; then
    echo "Error: AGENT_TYPE must be 'cursor' or 'claude', got: $AGENT_TYPE" >&2
    return 1
fi
```

## No Changes Required

These files already support environment variables through `helpers.sh`:

- `bin/build-feature` - sources helpers.sh, uses `run_agent()`
- `bin/gen-doc` - sources helpers.sh, uses `run_agent()`

Both scripts delegate to shared helpers; no individual changes needed.

## Testing Steps

| Test | Command | Expected |
|------|---------|----------|
| Default | `build-feature Test docs "test"` | Uses claude agent |
| Explicit claude | `AGENT_TYPE=claude build-feature Test docs "test"` | Uses claude |
| Explicit cursor | `AGENT_TYPE=cursor build-feature Test docs "test"` | Uses cursor-agent |
| Invalid | `AGENT_TYPE=invalid build-feature Test docs "test"` | Error exit 1 |
| Custom IDE | `IDE=vscode build-feature Test docs "test"` | Passes vscode |
| Custom model | `CURSOR_MODEL=opus-4.5-thinking AGENT_TYPE=cursor build-feature Test docs "test"` | Uses opus |

## Breaking Change Notice

Changing default `AGENT_TYPE` to "claude" breaks existing users without the claude CLI installed. Users depending on cursor must explicitly set:

```bash
export AGENT_TYPE=cursor
```

## Migration Path

1. Update helpers.sh (1 line change)
2. Update README.md documentation
3. Update TODOS.md to mark complete
4. Test with all scenarios above
5. Commit with message: "feat: default AGENT_TYPE to claude, document env vars"
