# Implementation Plan: Add Non-Interactive Mode to `run_agent`

Reference: [01-design.md](./01-design.md) | [02-research.md](./02-research.md)

---

## Summary

Add a `--no-interactive` flag to `run_agent()` in `bin/helpers.sh`. When set, append `--print` to the underlying CLI command. Update `bin/build-feature` stage 5 (implementation) to use it.

**Files to modify:** `bin/helpers.sh`, `bin/build-feature`
**Files unchanged:** `bin/gen-doc`

---

## Step 1: Modify `run_agent` in `bin/helpers.sh`

Add `--no-interactive` flag parsing as second argument. When present, append `--print` to the agent command.

**Current** (`bin/helpers.sh:42-65`):

```bash
run_agent() {
    local stage="$1"
    local prompt
    prompt=$(cat)
    # ...
    if [[ "$AGENT_TYPE" == "claude" ]]; then
        if ! echo "$prompt" | claude --dangerously-skip-permissions; then
```

**New:**

```bash
run_agent() {
    local stage="$1"
    local interactive=true

    if [[ "$2" == "--no-interactive" ]]; then
        interactive=false
    fi

    local prompt
    prompt=$(cat)

    if [[ -z "$prompt" ]]; then
        echo "Error: No prompt provided to run_agent" >&2
        return 1
    fi

    if [[ "$AGENT_TYPE" == "claude" ]]; then
        local cmd=(claude --dangerously-skip-permissions)
        if [[ "$interactive" == false ]]; then
            cmd+=(--print)
        fi
        if ! echo "$prompt" | "${cmd[@]}"; then
            echo "Error: claude agent failed at stage '$stage'" >&2
            return 1
        fi
    else
        local cmd=(cursor-agent --model "$CURSOR_MODEL")
        if [[ "$interactive" == false ]]; then
            cmd+=(--print)
        fi
        if ! echo "$prompt" | "${cmd[@]}"; then
            echo "Error: cursor-agent failed at stage '$stage'" >&2
            return 1
        fi
    fi

    echo "✓ Stage '$stage' complete"
}
```

**Key decisions:**
- Simple `$2` check — only one flag, no need for `getopts`
- Array-based command building avoids quoting issues
- `--dangerously-skip-permissions` kept alongside `--print` (per research: `--print` only skips trust dialog, not all permissions)

---

## Step 2: Update `bin/build-feature` Stage 5

**Current** (`bin/build-feature:131`):

```bash
echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" || exit 1
```

**New:**

```bash
echo "$IMPLEMENTATION_PROMPT" | run_agent "$STAGE" --no-interactive || exit 1
```

One-line change. All other stages remain interactive (no second argument).

---

## Step 3: No Changes to `bin/gen-doc`

All `run_agent` calls in `gen-doc` remain interactive (default behavior preserved).

---

## Verification

After implementation, verify:

1. **Interactive stages still work:** Run `build-feature` — stages 1-4 should behave identically (agent prompts, user interaction)
2. **Non-interactive implementation:** Stage 5 should run with `--print` — no user prompts, output to stdout
3. **Error propagation:** Kill agent mid-stage-5 — script should exit with error message
4. **Both agent types:** Test with `AGENT_TYPE=claude` and `AGENT_TYPE=cursor`

Quick smoke test:

```bash
# Verify --print flag is passed (dry run with ps or debug)
AGENT_TYPE=claude bash -x bin/build-feature test-feature docs "test prompt" 2>&1 | grep -- '--print'
```

---

## Checklist

- [ ] Modify `run_agent()` in `bin/helpers.sh` to accept `--no-interactive` flag
- [ ] Add `--print` to claude command when `--no-interactive` is set
- [ ] Add `--print` to cursor-agent command when `--no-interactive` is set
- [ ] Update `bin/build-feature` stage 5 call to pass `--no-interactive`
- [ ] Test interactive stages 1-4 are unchanged
- [ ] Test non-interactive stage 5 runs with `--print`
- [ ] Test with `AGENT_TYPE=claude`
- [ ] Test with `AGENT_TYPE=cursor`
- [ ] Test error propagation in non-interactive mode

---

## Detailed Checklist

See [04-checklist.md](./04-checklist.md) for the full implementation checklist with sub-tasks.
