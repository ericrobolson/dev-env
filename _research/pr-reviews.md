# PR review research

Research synthesis for building a practical, personal checklist for reviewing pull requests. This document gathers recurring ideas from the links in [`.tmp_PRREViews.md`](../.tmp_PRREViews.md) and adds primary guidance where the source list did not directly cover security and authorization.

The intended use is as a set of coaching prompts, not an automatic gate. A review should help decide whether a change is correct, safe, understandable, and supportable in its actual system context. It should also make clear which concerns need resolution before merge and which are suggestions.

## Main themes

### 1. Establish intent and scope before reviewing details

- Can I explain what user, product, or operational need this change addresses?
- Does the PR description connect the diff to the requirement, issue, or design decision?
- Does the change actually deliver that outcome, including the cases that are easy to miss?
- Is the PR small and focused enough to review coherently? If not, can it be split, or can the author identify the main design area to review first?
- Is this the right time and place to make this change, given the direction of the surrounding system?

Reviewing only the visible diff can miss omitted work, duplicate capabilities, and mismatches with the actual requirement. For larger or riskier changes, discuss the approach early enough to avoid expensive rework; avoid turning every small change into a pre-approval process.

### 2. Review behavior and design in the context of the system

- Does the change behave correctly for expected inputs, boundary cases, invalid states, and failures?
- What important behavior is absent from the diff: other call sites, related components, migrations, documentation, tests, or user flows?
- Does this reuse an existing capability, or introduce another implementation of the same concept?
- Is each responsibility in the right layer? Is policy or domain behavior leaking into UI code, transport handlers, or unrelated infrastructure?
- Are new files in the right part of the project, following established ownership and dependency boundaries? Does the change make code easier to find and maintain?
- Is the abstraction or complexity justified by a real current requirement, rather than a speculative future need?
- Does the change preserve existing contracts and behavior for callers, stored data, integrations, and supported clients?

Consistency with the codebase is valuable, but not when it perpetuates a design that this change can reasonably improve. Where several options are technically sound, accept the author’s choice instead of requiring the reviewer’s preferred implementation.

### 3. Examine policy, identity, and access control

- What policy determines whether this action is allowed, and where is that policy defined?
- Are authentication (who the caller is) and authorization (what that caller may do) treated separately?
- Are permissions checked for every relevant operation and resource, including object ownership, tenant boundaries, and administrative actions?
- Is the decisive check performed on a trusted server-side boundary? Could someone bypass a UI check by calling an API directly or changing request parameters?
- Are security-relevant values such as owner, role, price, or workflow state derived or validated from trusted server-side data rather than trusted from the client?
- Are invalid state transitions and unauthorized requests rejected safely, without partial writes, information leaks, or an unstable state?
- Are policy rules centralized enough to be applied consistently, while remaining understandable and testable?

Client-side checks can improve the user experience, but must not be the authority that grants access. Authorization should be enforced at the server, gateway, or equivalent trusted boundary. Test direct entry points as well as normal UI flows.

### 4. Decide what belongs in code versus configuration

- Is this value a stable invariant of the product, or does it reasonably vary by environment, deployment, tenant, operator, or policy?
- Are thresholds, URLs, feature switches, provider choices, and policy values hardcoded when they need operational control—or made configurable when variation would add needless complexity?
- Are defaults safe, documented, and suitable for production? What happens when configuration is missing, malformed, or inconsistent?
- Are credentials and other secrets supplied through an appropriate secret-management mechanism rather than committed to source or shipped to clients?
- Does configuration have validation, ownership, and change visibility appropriate to its risk?

Prefer the simplest source of truth that fits the real variation requirement. Configuration is not automatically better: every configurable option adds another state to validate, document, secure, and support.

### 5. Evaluate tests and evidence

- Do tests cover the new behavior and important regressions, including denial paths, edge cases, and failures?
- Would these tests fail if the production behavior were broken? Are assertions meaningful, or could the tests pass without proving the requirement?
- Are test names and failure messages useful when something breaks?
- Is coverage at the right level—unit, integration, end-to-end, or manual—without making tests brittle or needlessly broad?
- For UI, concurrency, or environment-sensitive behavior, is inspection or running the change needed in addition to automated tests?
- Were related tests, fixtures, scripts, docs, and supported implementations updated, rather than only the obvious changed file?

Tests are part of the change and need review themselves. A test count or green check alone does not demonstrate that the intended behavior is exercised.

### 6. Check production readiness and customer support

- What changes for users, administrators, support staff, or downstream systems?
- Is Customer Experience (CX), support, or the relevant product owner aware when the change affects customer workflows, behavior, or troubleshooting?
- Do support teams have the product behavior, known limitations, and troubleshooting information needed to answer likely questions?
- Are release notes, user communication, documentation, or training needed for a visible behavior change?
- Are errors handled usefully and safely? Can operators tell what happened without exposing sensitive data?
- Are appropriate logs, metrics, alerts, and audit records added for this change, without recording secrets or sensitive user data?
- Could the change affect latency, throughput, resource use, cost, availability, or data retention?
- Is rollout controlled where needed (for example, a feature flag or staged deployment), and is the flag itself observable and removable?
- Are production and staging differences understood? Can the change be rolled back or safely disabled, and what happens to data written while it is enabled?

Production readiness is part of feature correctness. New behavior may require new operational knowledge, monitoring, support preparation, release communication, or rollback handling even when the code and tests look sound.

### 7. Make review feedback clear and proportionate

- Is each comment tied to a concrete failure mode, maintainability concern, requirement, or established convention?
- Does the comment explain why the issue matters and give enough context to act on it?
- Have I separated must-fix findings from optional ideas, educational notes, and nits?
- Am I asking for a change because it materially improves correctness, security, usability, or code health—or only because I would have written it differently?
- Have I prioritized the most important findings so that critical issues are not buried in a long list of minor comments?
- Have I reviewed the author’s amendments and confirmed that important concerns were addressed?
- Have I acknowledged good decisions or useful improvements as well as problems?

Use review status consistently: if a concern means the change should not merge, make that blocking intent explicit. If the change is acceptable and comments are suggestions, say so. Aim for continuous improvement rather than perfection, and resolve disagreements with evidence, system conventions, or an appropriate design discussion.

## Suggested review flow

1. Read the PR description and requirement; clarify intent if it is missing.
2. Scan the overall diff and identify the core behavior and highest-risk areas.
3. Review design and placement in the context of neighboring code and system contracts.
4. Trace user input and permissions through trusted backend enforcement and failure handling.
5. Inspect tests and evidence, including what is missing from the diff.
6. Consider deployment, monitoring, rollback, support, and customer communication.
7. Leave a small set of specific, prioritized comments with clear blocking or non-blocking status.
8. Re-review material amendments and close the loop on outstanding concerns.

If the PR is too large or too unclear to review responsibly, ask for the requirement, a review guide, or a smaller set of independent changes rather than silently deferring or giving a cursory approval.

## Research basis

### Primary review guidance

- [Google Engineering Practices: What to Look For in a Code Review](https://google.github.io/eng-practices/review/reviewer/looking-for.html) — design, user functionality, complexity, tests, naming, comments, documentation, context, and code health.
- [Google Engineering Practices: The Standard of Code Review](https://google.github.io/eng-practices/review/reviewer/standard.html) — favor useful progress and continuous code-health improvement over perfection; distinguish nits and personal preference.
- [Google Engineering Practices: Writing Review Comments](https://google.github.io/eng-practices/review/reviewer/comments.html) — be courteous, explain reasoning, label optional feedback, and keep responsibility with the author.
- [Google Engineering Practices: Navigating a Review](https://google.github.io/eng-practices/review/reviewer/navigate.html) — take a broad view, inspect the central design first, and review the rest systematically.
- [Google Engineering Practices: Small CLs](https://google.github.io/eng-practices/review/developer/small-cls.html) — focused changes are easier to review; keep related tests with behavior changes.
- [OWASP Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html) — validate permissions on every request, enforce access at a trusted server-side boundary, and handle denials safely.
- [OWASP Business Logic Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Business_Logic_Security_Cheat_Sheet.html) — derive security-relevant values server-side, re-check ownership, and enforce workflow states.
- [OWASP Secure Code Review Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Secure_Code_Review_Cheat_Sheet.html) — inspect secrets, secure defaults, environment separation, logging, resource limits, business-rule enforcement, and security monitoring.

### Supplied articles and discussions

- [Tyler Cipriani, “Code Review Decision Fatigue”](https://tylercipriani.com/blog/2022/03/12/code-review-procrastination-and-clarity/) — reduce reviewer ambiguity with clear scope, smaller changes, useful tests, and shared review standards.
- [Sean Goedecke, “Mistakes I See Engineers Making in Their Code Reviews”](https://www.seangoedecke.com/good-code-reviews/) — review system context as well as the diff, avoid taste-based feedback, prioritize comments, and make blocking status explicit.
- [Scott Nonnenberg, “Top Ten Pull Request Review Mistakes”](https://blog.scottnonnenberg.com/top-ten-pull-request-review-mistakes/) — review against requirements, look for omissions, scrutinize tests and frontend behavior, and consider release notes, flags, monitoring, cost, and rollback.
- [Hacker News: “Commenting and approving pull requests”](https://news.ycombinator.com/item?id=47874613) — discussion of review status, trust, feedback clarity, conventions, and review latency.
- [Hacker News: “The primary purpose of code review is to find code that will be hard to maintain”](https://news.ycombinator.com/item?id=48759870) — discussion of maintainability as a review objective.
- [Hacker News: “Ask HN: How to deal with long vibe-coded PRs?”](https://news.ycombinator.com/item?id=45744209) — discussion of reviewability and the burden of large generated changes.
- [Hacker News: “Code review decision fatigue”](https://news.ycombinator.com/item?id=30665319), [“Ask HN: How do you keep code reviews from taking forever?”](https://news.ycombinator.com/item?id=32071776), and [“Ask HN: What is a common PR review time at your company?”](https://news.ycombinator.com/item?id=42656621) — discussions of review process, scope, and latency.
- [Hacker News: “Mistakes I see engineers making in their code reviews”](https://news.ycombinator.com/item?id=45701404), [“The Theatre of Pull Requests and Code Review”](https://news.ycombinator.com/item?id=45371283), and [“How to do a code review”](https://news.ycombinator.com/item?id=20890682) — community discussions around review quality and practice.
- Reddit discussions: [ExperiencedDevs](https://www.reddit.com/r/ExperiencedDevs/comments/1f4si9a/how_do_you_guys_do_your_prs/), [cscareerquestions](https://www.reddit.com/r/cscareerquestions/comments/za2ill/how_do_you_review_a_pull_request/), [rails](https://www.reddit.com/r/rails/comments/1ssvf6x/how_do_you_approach_pr_reviews_whats_your/), and [AskProgramming](https://www.reddit.com/r/AskProgramming/comments/1nzjiex/senior_engineers_how_do_you_review_pull_requests/).

Some supplied Hacker News pages returned retrieval errors during research: IDs `48329446`, `39288920`, `13504241`, `47540441`, and `49444984`. They remain in the original source list for later manual follow-up; no claims in this synthesis rely on their unavailable content.
