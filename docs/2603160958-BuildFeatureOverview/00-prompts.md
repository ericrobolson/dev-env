# Prompts

## Design

````
You are an expert product manager and designer.
You are to help me design a new feature which should do the following:
I want you to extend build-feature by adding a final step where it adds a summary of all the markdown files in the folder as well as code changes made. make it step 99 so it is always last and allows us to insert new pipeline stages

Explicitly outline any unknowns, questions, and any other details that are not clear.
The most important thing is to cover the entire user flow if relevant. Otherwise ask for high level features.
It is critical to include both the "happy" and "unhappy" flows in the design document.
Write out a testing plan and test cases for the feature.
However, do not go too deep at this stage. Stick to high level details.

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603160958-BuildFeatureOverview/01-design.md'.

Then open the file 'docs/2603160958-BuildFeatureOverview/01-design.md' in the IDE 'cursor' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
````

> To resume this session, run:
> `claude --resume 201506da-b10c-443c-8b18-aa7be2133bc8`

## Research

````
You are helpful assistant that is an expert in software development, project planning, product discovery, and architecture.

You are tasked to work on BuildFeatureOverview.

You are given the following design document to reference:
docs/2603160958-BuildFeatureOverview/01-design.md

You need to research each element in depth, listing out the intricacies, challenges, and other details.
List out any prior art and existing solutions found in the repo.
Analyze everything in great detail, providing deep research. Go through everything.
List out any relevant files and their file paths with a brief summary of how it is useful.

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603160958-BuildFeatureOverview/02-research.md'.

Then open the file 'docs/2603160958-BuildFeatureOverview/02-research.md' in the IDE 'cursor' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
````

> To resume this session, run:
> `claude --resume d5834cd8-b6d5-4c6f-947b-0cc7c4f5dee3`

## Plan

````
You are an expert at project management, development, architecture and design.
You prefer simple, robust solutions that are easy to maintain and extend.

You are tasked to work on BuildFeatureOverview.

You are given the following design document to reference:
docs/2603160958-BuildFeatureOverview/01-design.md

You are given the following research document to reference:
docs/2603160958-BuildFeatureOverview/02-research.md

Write a detailed document outlining how to implement this. include code snippets.
Ensure that both 'docs/2603160958-BuildFeatureOverview/01-design.md' and 'docs/2603160958-BuildFeatureOverview/02-research.md' are linked in the plan.

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603160958-BuildFeatureOverview/03-plan.md'.

Then open the file 'docs/2603160958-BuildFeatureOverview/03-plan.md' in the IDE 'cursor' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
````

> To resume this session, run:
> `claude --resume 82ed425b-6f23-4eb4-9f54-7f61a0f08491`

## Checklist

````
Given the following plan file in 'docs/2603160958-BuildFeatureOverview/03-plan.md', generate a checklist of tasks that need to be completed to implement the feature.
Be clear, concise, and explicit.
Split each task into smaller tasks if necessary.
Append the generated checklist to the end of the content in 'docs/2603160958-BuildFeatureOverview/03-plan.md'.

Be concise and to the point. Stick to facts. Be succinct and terse. Don't be verbose.

Output everything to the file 'docs/2603160958-BuildFeatureOverview/04-checklist.md'.

Then open the file 'docs/2603160958-BuildFeatureOverview/04-checklist.md' in the IDE 'cursor' so I can review it.

If possible, play an audio notification to alert me that the file is ready to review.
````

> To resume this session, run:
> `claude --resume 9c909657-0344-4197-81c2-817aa675d17e`

## Implementation

````
You are an expert at software development, project management, architecture and design.
You prefer simple, robust solutions that are easy to maintain and extend.
Make code modular wherever possible.
You are tasked to work on BuildFeatureOverview.
Implement the following plan document in 'docs/2603160958-BuildFeatureOverview/04-checklist.md'.

If possible, play an audio notification to alert me when everything is finished.
````

> To resume this session, run:
> `claude --resume 0942aa3f-d89a-47ec-b852-30a99a8cc2c4`

## Debug

````
You are an expert at debugging and fixing software issues.
You are tasked to work on BuildFeatureOverview.

Reference these documents for full context:
- Design: 'docs/2603160958-BuildFeatureOverview/01-design.md'
- Research: 'docs/2603160958-BuildFeatureOverview/02-research.md'
- Plan: 'docs/2603160958-BuildFeatureOverview/03-plan.md'
- Checklist: 'docs/2603160958-BuildFeatureOverview/04-checklist.md'

Do not run anything at this point. You are waiting on input from the user.

The user will describe bugs, issues, or changes they want to make found after implementation.
Fix each item as described.

Write all conversation output, including what was changed and why, to 'docs/2603160958-BuildFeatureOverview/05-debug.md'.
Each interaction should be recorded in 'docs/2603160958-BuildFeatureOverview/05-debug.md'.
````

> To resume this session, run:
> `claude --resume 830bd563-f22b-44be-ad92-4953b64df951`

