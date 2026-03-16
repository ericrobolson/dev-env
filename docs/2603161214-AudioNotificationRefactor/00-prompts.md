# Prompts

## Design

````
You are an expert product manager and designer.
You are to help me design a new feature which should do the following:
I want you to create a new script in bin that will play an audio sound on execution. Then update all existing executables to run that script wherever they tell an agent to play an audio sound

Explicitly outline any unknowns, questions, and any other details that are not clear.
The most important thing is to cover the entire user flow if relevant. Otherwise ask for high level features.
It is critical to include both the "happy" and "unhappy" flows in the design document.
Write out a testing plan and test cases for the feature.
However, do not go too deep at this stage. Stick to high level details.

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603161214-AudioNotificationRefactor/01-design.md'.

Then open the file 'docs/2603161214-AudioNotificationRefactor/01-design.md' in the IDE 'cursor' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
````

> To resume this session, run:
> `claude --resume c3eb1e38-0384-43ec-85f2-84cc19db0632`

## Research

````
You are helpful assistant that is an expert in software development, project planning, product discovery, and architecture.

You are tasked to work on AudioNotificationRefactor.

You are given the following design document to reference:
docs/2603161214-AudioNotificationRefactor/01-design.md

You need to research each element in depth, listing out the intricacies, challenges, and other details.
List out any prior art and existing solutions found in the repo.
Analyze everything in great detail, providing deep research. Go through everything.
List out any relevant files and their file paths with a brief summary of how it is useful.

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603161214-AudioNotificationRefactor/02-research.md'.

Then open the file 'docs/2603161214-AudioNotificationRefactor/02-research.md' in the IDE 'cursor' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
````

> To resume this session, run:
> `claude --resume dd9cef16-803a-43af-8068-3647b378de0c`

## Plan

````
You are an expert at project management, development, architecture and design.
You prefer simple, robust solutions that are easy to maintain and extend.

You are tasked to work on AudioNotificationRefactor.

You are given the following design document to reference:
docs/2603161214-AudioNotificationRefactor/01-design.md

You are given the following research document to reference:
docs/2603161214-AudioNotificationRefactor/02-research.md

Write a detailed document outlining how to implement this. include code snippets.
Ensure that both 'docs/2603161214-AudioNotificationRefactor/01-design.md' and 'docs/2603161214-AudioNotificationRefactor/02-research.md' are linked in the plan.

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603161214-AudioNotificationRefactor/03-plan.md'.

Then open the file 'docs/2603161214-AudioNotificationRefactor/03-plan.md' in the IDE 'cursor' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
````

> To resume this session, run:
> `claude --resume e0cdfc01-c3c1-4c7e-99ad-fc86c4f3cfe6`

## Checklist

````
Given the following plan file in 'docs/2603161214-AudioNotificationRefactor/03-plan.md', generate a checklist of tasks that need to be completed to implement the feature.
Be clear, concise, and explicit.
Split each task into smaller tasks if necessary.
Append the generated checklist to the end of the content in 'docs/2603161214-AudioNotificationRefactor/03-plan.md'.

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603161214-AudioNotificationRefactor/04-checklist.md'.

Then open the file 'docs/2603161214-AudioNotificationRefactor/04-checklist.md' in the IDE 'cursor' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
````

> To resume this session, run:
> `claude --resume 9766d437-35f6-4d90-a5ae-36588b3874f3`

## Implementation

````
You are an expert at software development, project management, architecture and design.
You prefer simple, robust solutions that are easy to maintain and extend.
Make code modular wherever possible.
You are tasked to work on AudioNotificationRefactor.
Implement the following plan document in 'docs/2603161214-AudioNotificationRefactor/04-checklist.md'.

If possible, play an audio notification to alert me when everything is finished.
````

> To resume this session, run:
> `claude --resume 87d76707-7ca0-4c2a-96eb-de8996559d70`

## Debug

````
You are an expert at debugging and fixing software issues.
You are tasked to work on AudioNotificationRefactor.

Reference these documents for full context:
- Design: 'docs/2603161214-AudioNotificationRefactor/01-design.md'
- Research: 'docs/2603161214-AudioNotificationRefactor/02-research.md'
- Plan: 'docs/2603161214-AudioNotificationRefactor/03-plan.md'
- Checklist: 'docs/2603161214-AudioNotificationRefactor/04-checklist.md'

Do not run anything at this point. You are waiting on input from the user.

The user will describe bugs, issues, or changes they want to make found after implementation.
Fix each item as described.

Write all conversation output, including what was changed and why, to 'docs/2603161214-AudioNotificationRefactor/05-debug.md'.
Each interaction should be recorded in 'docs/2603161214-AudioNotificationRefactor/05-debug.md'.
````

> To resume this session, run:
> `claude --resume 92dd6678-080c-4ba2-955e-a4b1b586be71`

## Overview

````
You are an expert technical writer.
You are tasked to write a pipeline overview for AudioNotificationRefactor.

Read all markdown files in 'docs/2603161214-AudioNotificationRefactor' in filename-sorted order, excluding '00-prompts.md'.
For each file, write a concise summary.

Then summarize all code changes made during this pipeline run.
Run: git diff 2d1f6a5a5606d3feac17361992d4a98aa1113c1b --stat
Run: git diff 2d1f6a5a5606d3feac17361992d4a98aa1113c1b
Run: git ls-files --others --exclude-standard

If the diff is large, summarize at a high level (files changed, nature of changes).
Do not include the contents of '00-prompts.md'.

Write a high-level feature summary at the top (what was built and why).

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603161214-AudioNotificationRefactor/99-overview.md'.

Then open the file 'docs/2603161214-AudioNotificationRefactor/99-overview.md' in the IDE 'cursor' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
````

> To resume this session, run:
> `claude --resume efa27c67-c6cf-4dae-bc32-41f2e4f8d362`

