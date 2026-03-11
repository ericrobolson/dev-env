# Implementation Checklist: Configurable Agent Environment Variables

## Code Changes

### bin/helpers.sh
- [ ] Change line 30: Update `AGENT_TYPE="${AGENT_TYPE:-cursor}"` to `AGENT_TYPE="${AGENT_TYPE:-claude}"`

### README.md
- [ ] Update Environment Variables table (lines 24-28): Change AGENT_TYPE default from "cursor" to "claude"
- [ ] Update Configuration section (lines 161-179): Update example export statements
- [ ] Remove outdated "hardcoded" claims
- [ ] Remove outdated TODO placeholder

### TODOS.md
- [ ] Mark item complete (lines 8-11): Check off "Update bin/build-feature and bin/gen-doc to be agent agnostic/configurable"
- [ ] Add completion notes: default changed, env vars documented

## Testing

- [ ] Test default: `build-feature Test docs "test"` → uses claude
- [ ] Test explicit claude: `AGENT_TYPE=claude build-feature Test docs "test"` → uses claude
- [ ] Test explicit cursor: `AGENT_TYPE=cursor build-feature Test docs "test"` → uses cursor-agent
- [ ] Test invalid: `AGENT_TYPE=invalid build-feature Test docs "test"` → exits with error
- [ ] Test custom IDE: `IDE=vscode build-feature Test docs "test"` → passes vscode
- [ ] Test custom model: `CURSOR_MODEL=opus-4.5-thinking AGENT_TYPE=cursor build-feature Test docs "test"` → uses opus
