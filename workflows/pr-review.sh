#!/usr/bin/env bash
set -euo pipefail

# A short, gated review flow based on _research/pr-reviews.md.
# Each section is opened with a yes/no applicability gate. A "no" on a
# section check is collected as an attention item for the reviewer.

declare -a SECTIONS=()
declare -a GATES=()
declare -a RELEVANCE=()
declare -a CHECK_SECTIONS=()
declare -a CHECKS=()
declare -a ATTENTION=()

add_section() {
    SECTIONS+=("$1")
    GATES+=("$2")
    RELEVANCE+=("$3")
}

add_check() {
    CHECK_SECTIONS+=("$1")
    CHECKS+=("$2")
}

add_section \
    '1. Intent and scope' \
    'Should I review intent and scope for this PR?' \
    'Relevant for every PR: understand the user, product, or operational need; check the description and whether the change is focused.'
add_check 0 'Can I explain the need this change addresses and how the diff delivers it?'
add_check 0 'Is the change focused and consistent with the surrounding product direction?'

add_section \
    '2. Behavior and omissions' \
    'Should I trace behavior and look for omissions in this PR?' \
    'Relevant when behavior changes, including edge cases, failures, related call sites, migrations, docs, or user flows.'
add_check 1 'Does the change handle expected inputs, boundary cases, invalid states, and failures?'
add_check 1 'Are related components, callers, data changes, and user flows covered where needed?'
add_check 1 'Does it preserve existing contracts for callers, stored data, integrations, and supported clients?'

add_section \
    '3. Design and contracts' \
    'Should I review design and system boundaries for this PR?' \
    'Relevant when responsibilities, dependencies, abstractions, or new files are introduced or moved.'
add_check 2 'Are responsibilities in the right layer and new code in the right part of the project?'
add_check 2 'Is the complexity justified by a current requirement, and does it fit or improve established patterns?'
add_check 2 'Am I focusing feedback on material issues rather than my personal implementation preference?'

add_section \
    '4. Security and permissions' \
    'Does this PR touch permissions, identity, sensitive data, or trusted input?' \
    'Relevant when a change handles authentication, authorization, ownership, tenant boundaries, sensitive values, or state transitions.'
add_check 3 'Are permissions enforced at a trusted boundary for every relevant operation and resource?'
add_check 3 'Are security-sensitive values derived or validated from trusted data, not accepted from the client?'
add_check 3 'Are unauthorized requests and invalid transitions rejected safely, without leaks or partial writes?'
add_check 3 'Do tests or inspection cover direct entry points as well as normal UI flows?'

add_section \
    '5. Tests and evidence' \
    'Should I review test coverage and other evidence for this PR?' \
    'Relevant when behavior changes or correctness depends on UI, concurrency, integrations, or environment-specific behavior.'
add_check 4 'Do tests meaningfully prove the changed behavior, including important regressions and failure paths?'
add_check 4 'Have I reviewed the tests themselves and checked that related fixtures, scripts, and supported implementations are updated?'
add_check 4 'Is additional inspection or a manual run needed beyond the automated tests?'

add_section \
    '6. Customer-visible behavior' \
    'Does this PR change behavior or content that customers can see or experience?' \
    'Relevant for changed workflows, UI, API behavior, notifications, performance, errors, or customer-facing documentation.'
add_check 5 'Is the customer-facing behavior understandable and useful, including error and empty states?'
add_check 5 'Are visible behavior changes documented or communicated where needed?'
add_check 5 'Could this change affect customer data, latency, availability, or other user expectations?'

add_section \
    '7. CX and support readiness' \
    'Does this PR change what CX or support teams may need to explain or troubleshoot?' \
    'Relevant when customer workflows, known limitations, troubleshooting steps, or support procedures change.'
add_check 6 'Do CX and support have the behavior and known limitations they need to answer likely questions?'
add_check 6 'Can support diagnose likely failures without exposing sensitive information?'
add_check 6 'Are support guidance, training, or escalation paths updated where needed?'

add_section \
    '8. Production readiness' \
    'Does this PR affect production configuration, operations, or rollout?' \
    'Relevant when changing secrets, configuration, provider choices, observability, resource use, deployment, or persisted data.'
add_check 7 'Are defaults safe, configuration validated, and secrets handled appropriately?'
add_check 7 'Are appropriate logs, metrics, alerts, or audit records available without recording sensitive data?'
add_check 7 'Is rollout, disablement, and rollback understood, including data written while the change is enabled?'

add_section \
    '9. Review feedback and follow-up' \
    'Should I check review comments and follow-up for this PR?' \
    'Relevant for every review: make findings actionable, distinguish blockers from suggestions, and re-check material amendments.'
add_check 8 'Are comments specific, prioritized, and clear about why the issue matters and what to do next?'
add_check 8 'Are blocking concerns explicit, with optional suggestions separated from must-fix findings?'
add_check 8 'Have I reviewed material amendments and closed the loop on outstanding concerns?'

ask_yes_no() {
    local prompt="$1"
    local answer

    while true; do
        printf '%s [y/n] ' "$prompt"
        if ! IFS= read -r answer; then
            printf '\nInput closed; ending review checklist.\n' >&2
            exit 1
        fi
        case "${answer,,}" in
            y|yes) REPLY='yes'; return 0 ;;
            n|no) REPLY='no'; return 0 ;;
            *) printf 'Please answer y, n, yes, or no.\n' ;;
        esac
    done
}

printf 'PR review checklist\n'
printf 'Answer yes or no. Section gates show when a focused set of checks is relevant.\n'

for i in "${!SECTIONS[@]}"; do
    printf '\n\n=== %s ===\n' "${SECTIONS[$i]}"
    printf 'Relevant when: %s\n' "${RELEVANCE[$i]}"
    ask_yes_no "${GATES[$i]}"
    [[ "$REPLY" == 'yes' ]] || continue

    for j in "${!CHECKS[@]}"; do
        [[ "${CHECK_SECTIONS[$j]}" == "$i" ]] || continue
        ask_yes_no "${CHECKS[$j]}"
        if [[ "$REPLY" == 'no' ]]; then
            ATTENTION+=("${SECTIONS[$i]}|${CHECKS[$j]}")
        fi
    done
done

printf '\n\n=== Review attention list ===\n'
if ((${#ATTENTION[@]} == 0)); then
    printf 'No checklist concerns recorded. Use your judgment when writing the PR review.\n'
else
    current_section=''
    for item in "${ATTENTION[@]}"; do
        section="${item%%|*}"
        question="${item#*|}"
        if [[ "$section" != "$current_section" ]]; then
            current_section="$section"
            printf '\n%s\n' "$current_section"
        fi
        printf '  - %s\n' "$question"
    done
fi

printf '\nChecklist complete. Write concrete findings in the PR, make blocking status explicit, and acknowledge useful decisions.\n'
