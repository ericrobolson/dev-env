# TODOs
A list of items to work on.

## Items
- [x] Convert 'one-shot' to 'gen-doc'
- [x] Update `gen-doc` to follow patterns in `build-feature` for agent agnosticism
- - [x] Maybe even combine functionality into a shared file that gets imported into both `gen-doc` and `build-feature`
- [x] Update  `bin/build-feature` and `bin/gen-doc` to be agent agnostic/configurable for everything.
- - [x] Agent type (cursor, claude, etc) - Default changed to `claude`, configurable via `AGENT_TYPE`
- - [x] Agent model (cursor) for implementation - Configurable via `CURSOR_MODEL`
- - [x] IDE - Configurable via `IDE` env var
- [x] Add non-interactive mode to `run_agent`
- [ ] Fix `Continue? (y/n)` prompt to be more user friendly. In reality it should only continue if prompt is y|yes|Y|Yes|YES. Otherwise it should prompt the user to enter 'y' or 'n' when ready.
- [ ] Add a 'debug' stage after implementation so user can continue to provide updates in the context.. Then write out all findings to a DEBUG.md file 
- [ ] Add splitting of logs to a file for post-mortem analysis. Use `tee` to split logs to a file and stdout.
- [ ] Add message to final prompt if they're in interactive mode to exit to continue to next stage.