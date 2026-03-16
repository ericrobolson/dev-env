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
- [x] Add a 'debug' stage after implementation so user can continue to provide updates in the context.. Then write out all findings to a DEBUG.md file
- [x] Add a \'00-prompts.md\' file to build-feature and gen-doc to store all prompts for reference/replay things. Denote each prompt by the header of the stage it is for. Append each new prompt to the file. Add a generic method in helpers.sh to append a prompt to the file/create it called \'append_prompt\'.
- [x] Add a feature to append 'claude resume' to the 00-prompts.md file after each agent call. So it has the prompt, then the 'claude --resume 7f7c56bf-f51f-4124-b4b4-5f1ec9ff3fc9' session id after it with a note on running it to respond.
- [x] Add a 'overview' section to the build-feature pipeline that adds a summary of all files and high level summary of the feature and the changes done.
- [ ] Make checklist stage non-interactive
- [ ] Remove mentions of cursor? Claude first 
- [ ] Add a 'prompt builder' which critiques prompts and points out stuff to improve
- [ ] Update build-feature to search for an existing 'POST-IMPLEMENTATION.md' doc and run it before the debug stage. If it doesn't exist, create one under an 'agent-rules' directory.
- [ ] Fix `Continue? (y/n)` prompt to be more user friendly. In reality it should only continue if prompt is y|yes|Y|Yes|YES. Otherwise it should prompt the user to enter 'y' or 'n' when ready.
- [ ] Add splitting of logs to a file for post-mortem analysis. Use `tee` to split logs to a file and stdout.
- [ ] Add message to final prompt if they're in interactive mode to exit to continue to next stage.