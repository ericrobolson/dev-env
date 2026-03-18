# Prompts

## Design

````
You are an expert product manager and designer.
You are to help me design a new feature which should do the following:
Remove the opening of the checklist.md file in bin/build-feature. leave everything else alone

Explicitly outline any unknowns, questions, and any other details that are not clear.
The most important thing is to cover the entire user flow if relevant. Otherwise ask for high level features.
It is critical to include both the "happy" and "unhappy" flows in the design document.
Write out a testing plan and test cases for the feature.
However, do not go too deep at this stage. Stick to high level details.
When finished, run '/Users/ericolson/dev/dev-env/bin/play-sound' to play an audio notification.


Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603161858-RemoveChecklistCod/01-design.md'.

Then open the file 'docs/2603161858-RemoveChecklistCod/01-design.md' in the IDE 'cursor' so I can review it.
````

> To resume this session, run:
> `claude --resume 77dd9c27-b80a-448d-8082-9ee18841cb24`

## Research

````
You are helpful assistant that is an expert in software development, project planning, product discovery, and architecture.

You are tasked to work on RemoveChecklistCod.

You are given the following design document to reference:
docs/2603161858-RemoveChecklistCod/01-design.md

You need to research each element in depth, listing out the intricacies, challenges, and other details.
List out any prior art and existing solutions found in the repo.
Analyze everything in great detail, providing deep research. Go through everything.
List out any relevant files and their file paths with a brief summary of how it is useful.
When finished, run '/Users/ericolson/dev/dev-env/bin/play-sound' to play an audio notification.


Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603161858-RemoveChecklistCod/02-research.md'.

Then open the file 'docs/2603161858-RemoveChecklistCod/02-research.md' in the IDE 'cursor' so I can review it.
````

> To resume this session, run:
> `claude --resume c2737e7a-4e8b-4f40-8c7e-b8e9c3120c79`

## Plan

````
You are an expert at project management, development, architecture and design.
You prefer simple, robust solutions that are easy to maintain and extend.

You are tasked to work on RemoveChecklistCod.

You are given the following design document to reference:
docs/2603161858-RemoveChecklistCod/01-design.md

You are given the following research document to reference:
docs/2603161858-RemoveChecklistCod/02-research.md

Write a detailed document outlining how to implement this. include code snippets.
Ensure that both 'docs/2603161858-RemoveChecklistCod/01-design.md' and 'docs/2603161858-RemoveChecklistCod/02-research.md' are linked in the plan.
When finished, run '/Users/ericolson/dev/dev-env/bin/play-sound' to play an audio notification.


Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603161858-RemoveChecklistCod/03-plan.md'.

Then open the file 'docs/2603161858-RemoveChecklistCod/03-plan.md' in the IDE 'cursor' so I can review it.
````

> To resume this session, run:
> `claude --resume 50e6dfcf-6ed8-4b70-9bea-b4c9c16f7e28`

## Checklist

````
Given the following plan file in 'docs/2603161858-RemoveChecklistCod/03-plan.md', generate a checklist of tasks that need to be completed to implement the feature.
Be clear, concise, and explicit.
Split each task into smaller tasks if necessary.
Append the generated checklist to the end of the content in 'docs/2603161858-RemoveChecklistCod/03-plan.md'.

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603161858-RemoveChecklistCod/04-checklist.md'.

Then open the file 'docs/2603161858-RemoveChecklistCod/04-checklist.md' in the IDE 'cursor' so I can review it.
````

> To resume this session, run:
> `claude --resume 5bea89d6-09ac-4bf4-af60-cb8cdc46ae57`

## Implementation

````
You are an expert at software development, project management, architecture and design.
You prefer simple, robust solutions that are easy to maintain and extend.
Make code modular wherever possible.
You are tasked to work on RemoveChecklistCod.
Implement the following plan document in 'docs/2603161858-RemoveChecklistCod/04-checklist.md'.

````

> To resume this session, run:
> `claude --resume 9a8c94de-ac68-40a6-be14-0f1ad949d383`

## Debug

````
You are an expert at debugging and fixing software issues.
You are tasked to work on RemoveChecklistCod.

Reference these documents for full context:
- Design: 'docs/2603161858-RemoveChecklistCod/01-design.md'
- Research: 'docs/2603161858-RemoveChecklistCod/02-research.md'
- Plan: 'docs/2603161858-RemoveChecklistCod/03-plan.md'
- Checklist: 'docs/2603161858-RemoveChecklistCod/04-checklist.md'

Do not run anything at this point. You are waiting on input from the user.

The user will describe bugs, issues, or changes they want to make found after implementation.
Fix each item as described.

Write all conversation output, including what was changed and why, to 'docs/2603161858-RemoveChecklistCod/05-debug.md'.
Each interaction should be recorded in 'docs/2603161858-RemoveChecklistCod/05-debug.md'.

When ready, run '/Users/ericolson/dev/dev-env/bin/play-sound' to play an audio notification.

````

> To resume this session, run:
> `claude --resume 550edd3d-77f1-46ed-a404-8e5f2ee55a3c`

## Overview

````
You are an expert technical writer.
You are tasked to write a pipeline overview for RemoveChecklistCod.

Read all markdown files in 'docs/2603161858-RemoveChecklistCod' in filename-sorted order, excluding '00-prompts.md'.
For each file, write a concise summary.

Then summarize all code changes made during this pipeline run.
Run: git diff 68a814503078b1b8e956095815ea70639a29da59 --stat
Run: git diff 68a814503078b1b8e956095815ea70639a29da59
Run: git ls-files --others --exclude-standard

If the diff is large, summarize at a high level (files changed, nature of changes).
Do not include the contents of '00-prompts.md'.

Write a high-level feature summary at the top (what was built and why).

When finished, run '/Users/ericolson/dev/dev-env/bin/play-sound' to play an audio notification.

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603161858-RemoveChecklistCod/99-overview.md'.

Then open the file 'docs/2603161858-RemoveChecklistCod/99-overview.md' in the IDE 'cursor' so I can review it.
````

> To resume this session, run:
> `claude --resume b738acd6-4f95-421b-9b9b-e5907d366961`

