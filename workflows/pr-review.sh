#!/usr/bin/env bash
set -euo pipefail

# A concise review flow based on _research/pr-reviews.md.
# Questions accept yes/no, free-form notes, or a blank response. All answers
# are summarized at the end for the reviewer to use while writing the review.

declare -a SECTIONS=()
declare -a CHECK_SECTIONS=()
declare -a CHECKS=()
declare -a RESPONSES=()

add_section() {
    SECTIONS+=("$1")
}

add_check() {
    CHECK_SECTIONS+=("$1")
    CHECKS+=("$2")
}

add_section '1. Intent and scope'
add_check 0 'Can I explain the need this change addresses, and does the PR connect the diff to that requirement or decision?'
add_check 0 'Does the change deliver the intended outcome with a focused scope that fits the product direction?'

add_section '2. Behavior and omissions'
add_check 1 'Does the change behave correctly for expected inputs, boundary cases, invalid states, and failures?'
add_check 1 'Are related callers, components, data changes, migrations, docs, and user flows handled where needed?'

add_section '3. Design and contracts'
add_check 2 'Are responsibilities, files, dependencies, and reused capabilities in the right system boundaries?'
add_check 2 'Is the complexity justified, are existing contracts preserved, and is feedback about material issues rather than preference?'

add_section '4. Security and permissions'
add_check 3 'Are authentication and authorization handled separately, with permissions enforced at a trusted boundary for each relevant operation and resource?'
add_check 3 'Are security-sensitive values derived or validated from trusted data, and are unauthorized requests or invalid transitions rejected safely?'
add_check 3 'Do tests or inspection cover direct entry points and denial paths, without leaks, partial writes, or unstable state?'

add_section '5. Tests and evidence'
add_check 4 'Do meaningful tests prove the changed behavior and important regressions, including edge cases and failure paths?'
add_check 4 'Is the evidence at the right level, with related fixtures, scripts, supported implementations, or manual inspection covered where needed?'

add_section '6. Customer-visible behavior'
add_check 5 'Are customer-facing workflows, content, errors, and empty states understandable, with visible changes communicated where needed?'
add_check 5 'Are effects on customer data, latency, availability, and user expectations understood and acceptable?'

add_section '7. CX and support readiness'
add_check 6 'Do CX and support have the behavior, known limitations, and troubleshooting guidance they need?'
add_check 6 'Can support diagnose likely failures safely, with training or escalation paths updated where needed?'

add_section '8. Production readiness'
add_check 7 'Are configuration choices appropriate to real variation, with safe defaults, validation, and secret handling?'
add_check 7 'Are operational signals adequate, and could resource use, cost, availability, or data retention change?'
add_check 7 'Is rollout, disablement, and rollback understood, including effects on data written while enabled?'

add_section '9. Review feedback and follow-up'
add_check 8 'Are comments concrete, prioritized, and actionable, with blockers clearly separated from optional suggestions?'
add_check 8 'Have I re-reviewed material amendments, closed outstanding concerns, and acknowledged useful decisions?'

ask_question() {
    local prompt="$1"
    local answer

    printf '%s\n> ' "$prompt"
    if ! IFS= read -r answer; then
        printf '\nInput closed; ending review checklist.\n' >&2
        exit 1
    fi
    REPLY="$answer"
}

printf 'PR review checklist\n'
printf 'Enter yes or no, add a note, or press Enter to skip. Questions run in a fixed sequence.\n'

for i in "${!SECTIONS[@]}"; do
    printf '\n\n=== %s ===\n' "${SECTIONS[$i]}"
    for j in "${!CHECKS[@]}"; do
        [[ "${CHECK_SECTIONS[$j]}" == "$i" ]] || continue
        ask_question "${CHECKS[$j]}"
        RESPONSES+=("$REPLY")
    done
done

printf '\n\n=== Review summary ===\n'
current_check=0
for i in "${!SECTIONS[@]}"; do
    printf '\n%s\n' "${SECTIONS[$i]}"
    for j in "${!CHECKS[@]}"; do
        [[ "${CHECK_SECTIONS[$j]}" == "$i" ]] || continue
        response="${RESPONSES[$current_check]}"
        printf '  - %s\n' "${CHECKS[$j]}"
        if [[ -n "${response//[[:space:]]/}" ]]; then
            printf '    %s\n' "$response"
        fi
        current_check=$((current_check + 1))
    done
done

printf '\nChecklist complete. Write concrete findings in the PR, make blocking status explicit, and acknowledge useful decisions.\n'
