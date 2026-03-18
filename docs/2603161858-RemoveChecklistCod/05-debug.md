# Debug Log

## Issue 1: Play sound and open Cursor after implementation stage

**Request**: Play a sound when building code is done and open Cursor in the current directory for the implementation stage.

**Change**: Added `$PLAY_SOUND` and `$IDE .` after Stage 5 (Implementation) completes in `bin/build-feature`.

**File**: `bin/build-feature`, after line 164

**Before**:
```bash
echo "==> Implementation completed"
```

**After**:
```bash
echo "==> Implementation completed"
$PLAY_SOUND
$IDE .
```

**Why**: The implementation stage runs non-interactively (`--no-interactive`), so there's no notification when it finishes. Adding the sound alerts the user, and opening Cursor lets them review the generated code before the debug session begins.
