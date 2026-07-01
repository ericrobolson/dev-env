---
name: f-pseudocode
description: "Transform rules, formulas, or logic descriptions into clean procedural pseudocode in a single file. Use when user says 'pseudocode', 'f-pseudocode', 'write pseudocode', 'convert rules to pseudocode', or 'formalize these rules'."
---

# Pseudocode Generator

Transform a set of rules, formulas, or logic descriptions into a single procedural pseudocode file.

## Gather Input

If the user has already provided the rules/formulas (pasted text, file path, or document reference), skip this step.

Otherwise, ask the user for the rules to formalize. Accept any of:
- Pasted text (bullet points, natural language, math notation)
- A file path to read (markdown, PDF, text, doc-digest output)
- A reference to an existing document in the repo

Suggestions to offer:
1. "Paste your rules here and I'll formalize them"
2. "Point me to a file — e.g., `docs/combat_rules.md`"
3. "Reference an existing doc-digest — e.g., `doc-digest/some-book/`"

## Derive Output Name

From the subject matter of the rules, derive a PascalCase name for the output file (e.g., `CombatResolution`, `InventorySlots`, `SpellCasting`). This becomes the `{DerivedName}` in the filename.

## Detect Output Location

Determine where to write the file:

1. If the user's context references a specific project under `projects/<name>/`, use `projects/<name>/pseudo/`.
2. Otherwise, use repo-root `pseudo/`.

Run:
```bash
mkdir -p <target_dir>
```

Generate the timestamp:
```bash
date +%y%m%d-%H%M%S
```

Final path: `<target_dir>/<YYMMDD-HHMMSS>_<DerivedName>.pseudo`

## Generate Pseudocode

Write a single `.pseudo` file that translates the user's rules into procedural pseudocode. Follow these rules exactly:

### Syntax Style
- C-inspired with modern Rust/C# touches: `fn`, `struct`, `enum`, `let`, `->` return types, `match` expressions.
- Use Rust-style enums (tagged unions) where the rules describe variants or categories.
- Use `component` to declare data that attaches to entities (see Entity-Component Model below).
- Use `struct` for plain compound data that is not entity-attached.
- Use `fn` for functions with explicit parameter types and return types — systems operate on queries over components.
- Control flow: `if/else`, `for`, `while`, `match`, `return`.
- No semicolons required — keep it clean.

### Entity-Component Model

**Everything is an entity.** Players, hand zones, card decks, discard piles, boards, tokens, cursors, UI panels — if it has identity, it is an entity. Entities are opaque IDs; all data lives in components.

- **`entity`** — declare entity archetypes to show what components they typically carry (documentation, not enforcement).
- **`component`** — a named bag of fields that attaches to an entity. Components are pure data — no functions, no logic.
- **`system`** — a function that queries entities by their component signature and operates on them. All game logic lives in systems.
- **Relationships** — model ownership, containment, and references between entities with component fields that hold entity IDs (e.g., `owner: EntityId`, `parent_zone: EntityId`, `contents: [EntityId]`).

When in doubt about whether something should be an entity or a plain value, prefer entity. A card deck is an entity with a `DeckContents` component, not a `Vec<Card>` field on a player struct.

### File Structure (top to bottom)
1. **Header comment** — one line: the derived name and a one-sentence summary of what these rules describe.
2. **Constants** — every semantically meaningful literal from the rules becomes a named `const` at the top of the file. No magic numbers anywhere in the pseudocode.
3. **Enums** — any variant types or categories from the rules.
4. **Components** — data that attaches to entities, declared with `component`.
5. **Entity archetypes** — declared with `entity`, listing typical component composition.
6. **Systems** — functions declared with `system` that query entities by component signature. One system per rule or logical grouping.
7. **Setup / `fn main()`** — if the rules describe a process with a clear entry point, include a `main` that spawns initial entities and runs systems in order. Omit if the rules are purely declarative with no sequencing.

### Hard Constraints
- **No executable code.** This is a blueprint, not a program. Do not produce compilable C, Rust, Python, or anything else.
- **No OOP.** No classes, inheritance, polymorphism, methods, or `self`/`this`. Components, systems, enums, and control flow only.
- **No editorializing.** Faithfully represent the input rules. Do not invent behavior, add edge cases the user didn't specify, or "improve" the logic.
- **No comments explaining the pseudocode.** The pseudocode is the documentation. Only preserve annotations if the user's original rules contained them.
- **No monolith entities.** If a concept has sub-parts with independent identity (a hand of cards, a grid of tiles), each sub-part is its own entity linked via an EntityId reference — not a nested collection on the parent.
- **No magic numbers.** Every numeric or string literal with semantic meaning becomes a named `const` in the constants block. The only bare literals allowed are `0`, `1`, `true`, `false`, and empty collections.
- **No long functions.** No function or system body exceeds 40 lines. When a `match` or `if/else` chain has branches with non-trivial logic (more than 1–2 lines), extract each branch into a named function so the dispatch block reads as a table of one-liners.
- **DRY.** When two or more code paths share more than 5 lines of identical structure differing only in parameters, extract a shared function parameterized on the differences.
- **Section dividers.** Group systems into labeled sections using `-- SECTION_NAME` comment dividers (e.g., `-- SETUP`, `-- COMBAT`, `-- SPAWNING`, `-- PROGRESSION`). Each section groups related systems by subsystem.
- **Event-triggered checks at the trigger site.** When the source rules specify an event-triggered condition (e.g., "win when X happens"), check it at the point of the triggering action, not deferred to end-of-phase or end-of-turn.
- **Early return.** Prefer guard clauses and early returns over nested if/else chains. Check failure conditions and bail out at the top of a function rather than wrapping the happy path in a deep conditional.
- **No black-box types.** Every type that appears in a component field, function parameter, or return type must have a corresponding `enum`, `component`, `entity`, or `struct` declaration somewhere in the file. `Entity` and `EntityId` are built-in ECS primitives and do not need declarations.
- **Discriminate distinct roles.** When a single struct serves multiple distinct roles (weapon vs spell vs armor), add a category enum field or split into separate types. If the source rules treat them differently in any context, they need a discriminator.
- **Fields match usage.** Every field read by a system must exist on the component it reads from. After writing systems, back-check that all field accesses resolve to declared fields.
- **Correct relationship cardinality.** When modeling a relationship between entities, verify the cardinality matches the domain (1:1, 1:N, N:N) and declare both sides.
- **Single source of truth.** Each fact has one canonical location. If an entity stores a reference to its container, the container must not also store a redundant list of contents. Pick one and derive the other.
- **Honest field names.** Field names must describe what the value *is*, not what it will become. If a value is an input to further calculation, name it accordingly (e.g., `base_count` not `count`).
- **Closed sets are enums.** When a field's possible values form a known closed set, declare an enum rather than using a string or int.
- **No magic string identifiers.** When a named concept is referenced in multiple systems (skill names, action types, status effects), define it as an enum and reference the variant — never use raw strings as identifiers.
- **No identity operations.** After generating, scan for identity operations (`- 0`, `* 1`, `+ 0`, `== true`) and resolve each as either a real bug or remove the dead operation.
- **Symmetric side effects.** When parallel code paths (melee/ranged, buy/sell, spawn/despawn) perform the same logical action, verify they invoke the same side effects. Asymmetry requires an explicit justification from the source rules.
- **Mirror source language.** Pseudocode operations should mirror the source rules' language. If the rules say "+1 to each die result", model it as a bonus added during rolling, not as an equivalent threshold subtraction — traceability to source rules takes priority over mathematical equivalence.
- **Loop interrupts.** When a system performs a multi-step loop (move N zones, draw N cards), check for interrupting conditions within the loop body, not only before/after. If an interrupt is possible, the function must return status indicating whether it completed.
- **Variable conditions as data.** When the source rules define variable conditions (win/loss, triggers, quest-specific logic), model them as data — e.g., `fn(GameState) -> bool` predicates on a config entity — not hardcoded if-chains in core systems.
- **Modifier pipelines over scattered conditionals.** When the source rules define a category of effects that share a pattern (skills, buffs, modifiers), model them as a modifier pipeline with phase tags (e.g., `PreRoll`, `PostRoll`, `OnHit`, `OnMove`) rather than scattering conditional checks across core systems.
- **Event dispatch for triggered abilities.** When the source rules describe reactive or triggered abilities ("after doing X, you may Y"), model an event dispatch system — `on_event(EventType, handler)` — so triggered effects are declared at definition site rather than wired into the triggering system.
- **Persistent board state as components.** When the source rules describe persistent state that accumulates across turns (tokens, counters, terrain effects), model it as a component on the relevant zone/entity and integrate it into the action dispatch, not as a standalone disconnected system.
- **Explicit action cost model.** When the source rules distinguish between resource-costing and free instances of the same action type, model the cost/free distinction explicitly in the activation context rather than treating all actions uniformly.
- **No orphaned declarations.** Every enum, struct, component, and entity must be referenced by at least one system or function. If a declaration exists only for documentation, annotate it explicitly.
- **All declarations in the preamble.** All enums, structs, and components must be declared in their designated file-structure section at the top. No inline type declarations mid-file inside systems or sections.
- **Sections follow the call graph.** Order section dividers so that sections appear before they are first called, enabling top-down reading. If A calls B, section B precedes section A.
- **No overlapping enums.** When two enums share a variant name or concept, either unify them into one enum, or make one reference the other explicitly. Silently parallel enums with overlapping semantics create ambiguity.
- **Every called function must be defined.** The "no black-box types" rule extends to functions — every function invoked in the file must have a corresponding `fn` or `system` definition. Platform and IO operations (user input, display, random) use a `HOST_` prefix (e.g., `HOST_read_line`, `HOST_prompt_choice`, `HOST_random`) and do not need definitions — the prefix marks them as external.
- **Consistent notation.** When using bracket notation `[Component]` or any other shorthand, define its meaning once at the top of the file and use it uniformly. Never overload the same notation to mean "query", "single entity", and "list" in different contexts.
- **Nullability is explicit.** If a field can be absent or null, mark it with `?` (e.g., `parent: EntityId?`). If a field is non-nullable, never assign `None` to it. Mismatches between declaration and usage are errors.
- **Consistent parameter types for symmetric functions.** When two functions serve the same conceptual role from different directions (e.g., `seats_left_from` / `seats_right_from`), their parameter and return types must match.
- **Prefer iteration over recursion.** Use loops instead of recursive calls unless the data is inherently tree-shaped (e.g., walking a tree hierarchy). Recursive calls on flat player lists or sequential chains should be rewritten as iteration.
- **Explicit type discrimination.** Never disambiguate types by positional context, traversal order, or secondary field combinations. Use an explicit type tag or enum variant so classification is unambiguous regardless of ordering.
- **No dead code.** Every declared field, parameter, and variable must be read or used. Parameters hardcoded to a single value at every call site should be removed and inlined. Fields declared but never checked in any system should be removed.
- **One concern per loop.** When a loop body combines multiple independent concerns (filtering, forcing, ordering, tracking), extract each concern into its own function called from within the loop or split into sequential passes.

### Example Fragment

```
const MIN_DAMAGE = 1
const CRIT_ROLL_MIN = 1
const CRIT_ROLL_MAX = 100
const CRIT_MULTIPLIER = 2

enum DamageType
    Physical
    Magical
    Elemental(Element)

component Health
    current: int
    max: int

component Attack
    power: int
    crit_chance: int
    damage_type: DamageType

component Defense
    armor: int

component Equipment
    weapon: EntityId

component WeaponStats
    power: int
    damage_type: DamageType

entity Player
    Health, Attack, Defense, Equipment

entity Weapon
    WeaponStats

system resolve_attack(attacker: [Attack, Equipment], defender: [Health, Defense], weapons: [WeaponStats])
    let weapon = weapons.get(attacker.equipment.weapon)
    let base = attacker.attack.power + weapon.power
    let raw = max(MIN_DAMAGE, base - defender.defense.armor)

    let crit_roll = random(CRIT_ROLL_MIN, CRIT_ROLL_MAX)
    if crit_roll <= attacker.attack.crit_chance
        defender.health.current -= raw * CRIT_MULTIPLIER
    else
        defender.health.current -= raw
```

## Post-creation

Verify the file was written:
```bash
test -f <output_path> && wc -l <output_path>
```

## Audit

Before returning to the user, spawn a **fork** subagent to audit the generated pseudocode against the source rules. The fork prompt must include:

1. The path to the generated `.pseudo` file.
2. The original source material (file path, pasted text reference, or inline content — whatever was used as input).

Fork prompt template:

> Audit the pseudocode at `<output_path>` against the source rules.
>
> Source: <source material or path to it>
>
> Read both the source rules and the generated pseudocode. Check for:
> - **Missing rules**: any rule, mechanic, formula, or condition present in the source that has no corresponding system, component, or logic in the pseudocode.
> - **Incomplete translations**: rules that are referenced but only partially implemented (e.g., a modifier mentioned but never applied, an enum variant declared but never matched).
> - **Structural violations**: any breach of the Hard Constraints from the skill (magic numbers, missing declarations, orphaned types, dead code, black-box types, etc.).
>
> If you find missing or incomplete functionality:
> 1. Edit the `.pseudo` file directly to add the missing pieces, following the same syntax style and file structure rules.
> 2. Do not remove or restructure existing correct pseudocode — only add what's missing or fix what's incomplete.
>
> If everything is covered, report "Audit clean — no missing functionality."
>
> End with a one-line summary of what was added or confirmed.

Wait for the fork to complete. Include its summary in your final report to the user alongside the file path and line count.
