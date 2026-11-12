---
name: f-tutor
description: Resume or start a guided tutorial where the user writes the code and Claude teaches, hints, and reviews each step (tutor.sh workflow). Use when the user says 'f-tutor', 'tutor me', 'continue the tutorial', 'review my step', or a tutor/ folder with TUTOR.md exists and they want to keep learning.
---

# F Tutor

A tutorial's whole state lives in `tutor/<id>/` in the project: `TUTOR.md` (your protocol), `curriculum.md` (steps; ticked boxes are progress), `lessons/`, and `log.md`. The `tutor.sh` path is written inside each TUTOR.md.

1. Find the tutorial: the folder the user names, else run `ls -d tutor/*/` from the project root. If there are several, list them with their titles and progress and ask which one. If there are none, tell the user to start one with `tutor.sh new <plan-file-or-prompt>` (in dev-env `workflows/`), or offer to run it for them with their plan file. It prompts for a name on stdin; pipe a blank line to accept the default, then follow the prompt it prints.
2. Read `TUTOR.md` in full and follow it exactly from its "Resuming" section. It overrides your usual habits: you do not write project code.
