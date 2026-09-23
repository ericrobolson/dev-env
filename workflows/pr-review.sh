#!/usr/bin/env bash
set -euo pipefail

# Interactive prompts based on _research/pr-reviews.md, ordered by its
# Suggested review flow.

declare -a SECTIONS=()
declare -a QUESTIONS=()
declare -a ANSWERS=()

add_question() {
    SECTIONS+=("$1")
    QUESTIONS+=("$2")
}

add_question '1. Read the PR description and requirement; clarify intent if it is missing.' 'Can I explain what user, product, or operational need this change addresses?'
add_question '1. Read the PR description and requirement; clarify intent if it is missing.' 'Does the PR description connect the diff to the requirement, issue, or design decision?'
add_question '1. Read the PR description and requirement; clarify intent if it is missing.' 'Does the change actually deliver that outcome, including the cases that are easy to miss?'
add_question '1. Read the PR description and requirement; clarify intent if it is missing.' 'Is the PR small and focused enough to review coherently? If not, can it be split, or can the author identify the main design area to review first?'
add_question '1. Read the PR description and requirement; clarify intent if it is missing.' 'Is this the right time and place to make this change, given the direction of the surrounding system?'

add_question '2. Scan the overall diff and identify the core behavior and highest-risk areas.' 'Does the change behave correctly for expected inputs, boundary cases, invalid states, and failures?'
add_question '2. Scan the overall diff and identify the core behavior and highest-risk areas.' 'What important behavior is absent from the diff: other call sites, related components, migrations, documentation, tests, or user flows?'
add_question '2. Scan the overall diff and identify the core behavior and highest-risk areas.' 'Does this reuse an existing capability, or introduce another implementation of the same concept?'
add_question '2. Scan the overall diff and identify the core behavior and highest-risk areas.' 'Is the abstraction or complexity justified by a real current requirement, rather than a speculative future need?'
add_question '2. Scan the overall diff and identify the core behavior and highest-risk areas.' 'Does the change preserve existing contracts and behavior for callers, stored data, integrations, and supported clients?'

add_question '3. Review design and placement in the context of neighboring code and system contracts.' 'Is each responsibility in the right layer? Is policy or domain behavior leaking into UI code, transport handlers, or unrelated infrastructure?'
add_question '3. Review design and placement in the context of neighboring code and system contracts.' 'Are new files in the right part of the project, following established ownership and dependency boundaries? Does the change make code easier to find and maintain?'
add_question '3. Review design and placement in the context of neighboring code and system contracts.' 'Is consistency with the codebase helping here, without perpetuating a design this change can reasonably improve?'
add_question '3. Review design and placement in the context of neighboring code and system contracts.' 'Where several options are technically sound, can I accept the author’s choice instead of requiring my preferred implementation?'

add_question '4. Trace user input and permissions through trusted backend enforcement and failure handling.' 'What policy determines whether this action is allowed, and where is that policy defined?'
add_question '4. Trace user input and permissions through trusted backend enforcement and failure handling.' 'Are authentication (who the caller is) and authorization (what that caller may do) treated separately?'
add_question '4. Trace user input and permissions through trusted backend enforcement and failure handling.' 'Are permissions checked for every relevant operation and resource, including object ownership, tenant boundaries, and administrative actions?'
add_question '4. Trace user input and permissions through trusted backend enforcement and failure handling.' 'Is the decisive check performed on a trusted server-side boundary? Could someone bypass a UI check by calling an API directly or changing request parameters?'
add_question '4. Trace user input and permissions through trusted backend enforcement and failure handling.' 'Are security-relevant values such as owner, role, price, or workflow state derived or validated from trusted server-side data rather than trusted from the client?'
add_question '4. Trace user input and permissions through trusted backend enforcement and failure handling.' 'Are invalid state transitions and unauthorized requests rejected safely, without partial writes, information leaks, or an unstable state?'
add_question '4. Trace user input and permissions through trusted backend enforcement and failure handling.' 'Are policy rules centralized enough to be applied consistently, while remaining understandable and testable?'
add_question '4. Trace user input and permissions through trusted backend enforcement and failure handling.' 'Do tests or inspection cover direct entry points as well as normal UI flows?'

add_question '5. Inspect tests and evidence, including what is missing from the diff.' 'Do tests cover the new behavior and important regressions, including denial paths, edge cases, and failures?'
add_question '5. Inspect tests and evidence, including what is missing from the diff.' 'Would these tests fail if the production behavior were broken? Are assertions meaningful, or could the tests pass without proving the requirement?'
add_question '5. Inspect tests and evidence, including what is missing from the diff.' 'Are test names and failure messages useful when something breaks?'
add_question '5. Inspect tests and evidence, including what is missing from the diff.' 'Is coverage at the right level—unit, integration, end-to-end, or manual—without making tests brittle or needlessly broad?'
add_question '5. Inspect tests and evidence, including what is missing from the diff.' 'For UI, concurrency, or environment-sensitive behavior, is inspection or running the change needed in addition to automated tests?'
add_question '5. Inspect tests and evidence, including what is missing from the diff.' 'Were related tests, fixtures, scripts, docs, and supported implementations updated, rather than only the obvious changed file?'
add_question '5. Inspect tests and evidence, including what is missing from the diff.' 'Have I reviewed the tests themselves, rather than relying on a test count or green check alone?'

add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Is this value a stable invariant of the product, or does it reasonably vary by environment, deployment, tenant, operator, or policy?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Are thresholds, URLs, feature switches, provider choices, and policy values hardcoded when they need operational control—or made configurable when variation would add needless complexity?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Are defaults safe, documented, and suitable for production? What happens when configuration is missing, malformed, or inconsistent?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Are credentials and other secrets supplied through an appropriate secret-management mechanism rather than committed to source or shipped to clients?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Does configuration have validation, ownership, and change visibility appropriate to its risk?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'What changes for users, administrators, support staff, or downstream systems?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Is Customer Experience (CX), support, or the relevant product owner aware when the change affects customer workflows, behavior, or troubleshooting?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Do support teams have the product behavior, known limitations, and troubleshooting information needed to answer likely questions?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Are release notes, user communication, documentation, or training needed for a visible behavior change?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Are errors handled usefully and safely? Can operators tell what happened without exposing sensitive data?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Are appropriate logs, metrics, alerts, and audit records added for this change, without recording secrets or sensitive user data?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Could the change affect latency, throughput, resource use, cost, availability, or data retention?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Is rollout controlled where needed (for example, a feature flag or staged deployment), and is the flag itself observable and removable?'
add_question '6. Consider deployment, monitoring, rollback, support, and customer communication.' 'Are production and staging differences understood? Can the change be rolled back or safely disabled, and what happens to data written while it is enabled?'

add_question '7. Leave a small set of specific, prioritized comments with clear blocking or non-blocking status.' 'Is each comment tied to a concrete failure mode, maintainability concern, requirement, or established convention?'
add_question '7. Leave a small set of specific, prioritized comments with clear blocking or non-blocking status.' 'Does the comment explain why the issue matters and give enough context to act on it?'
add_question '7. Leave a small set of specific, prioritized comments with clear blocking or non-blocking status.' 'Have I separated must-fix findings from optional ideas, educational notes, and nits?'
add_question '7. Leave a small set of specific, prioritized comments with clear blocking or non-blocking status.' 'Am I asking for a change because it materially improves correctness, security, usability, or code health—or only because I would have written it differently?'
add_question '7. Leave a small set of specific, prioritized comments with clear blocking or non-blocking status.' 'Have I prioritized the most important findings so that critical issues are not buried in a long list of minor comments?'
add_question '7. Leave a small set of specific, prioritized comments with clear blocking or non-blocking status.' 'Have I acknowledged good decisions or useful improvements as well as problems?'

add_question '8. Re-review material amendments and close the loop on outstanding concerns.' 'Have I reviewed the author’s amendments and confirmed that important concerns were addressed?'
add_question '8. Re-review material amendments and close the loop on outstanding concerns.' 'Have I used review status consistently, making blocking intent explicit when a concern means the change should not merge?'
add_question '8. Re-review material amendments and close the loop on outstanding concerns.' 'If the change is acceptable and comments are suggestions, have I made that clear?'
add_question '8. Re-review material amendments and close the loop on outstanding concerns.' 'If the PR is too large or unclear to review responsibly, have I asked for the requirement, a review guide, or a smaller set of independent changes?'

printf 'PR review checklist\n'
printf 'Answer each prompt with y or n. Your answers will be summarized at the end.\n'

current_section=''
for i in "${!QUESTIONS[@]}"; do
    if [[ "${SECTIONS[$i]}" != "$current_section" ]]; then
        current_section="${SECTIONS[$i]}"
        printf '\n\n=== %s ===\n' "$current_section"
    fi

    while true; do
        printf '\n%s [y/n] ' "${QUESTIONS[$i]}"
        if ! IFS= read -r answer; then
            printf '\nInput closed; ending review checklist.\n' >&2
            exit 1
        fi

        case "$answer" in
            [yY])
                ANSWERS+=(Y)
                break
                ;;
            [nN])
                ANSWERS+=(N)
                break
                ;;
            *)
                printf "Please answer 'y' or 'n'.\n"
                ;;
        esac
    done
done

printf '\n\n=== Review summary ===\n'
current_section=''
for i in "${!QUESTIONS[@]}"; do
    if [[ "${SECTIONS[$i]}" != "$current_section" ]]; then
        current_section="${SECTIONS[$i]}"
        printf '\n%s\n' "$current_section"
    fi
    printf '  [%s] %s\n' "${ANSWERS[$i]}" "${QUESTIONS[$i]}"
done

printf '\nChecklist complete.\n'
