---
name: seed-craft
description: Capture insights as structured seeds during governance sessions. Triggers on insight emergence, guides type selection, produces valid seed.yaml.
---

# Seed Craft: Capturing Insights

## Overview

This skill activates when an insight emerges that's worth preserving. It transforms ephemeral session knowledge into structured seeds that can later become articles.

**The goal:** Capture the insight while it's fresh, in a format that survives session boundaries.

## When to Invoke

Recognize seed-worthy moments:

- **"That's interesting..."** — A pattern noticed for the first time
- **"I keep doing this..."** — A technique that's become habitual
- **"The way I think about this is..."** — A framework crystallizing
- **"Remember when..."** — A story with transferable lessons
- **"This tool saved me..."** — A utility worth documenting

**The Gratitude Test:** Would someone be grateful to learn this? If yes, seed it.

## The Capture Protocol

### Phase 1: Identify the Type

Ask: "What kind of insight is this?"

| If it feels like... | The type is... |
|---------------------|----------------|
| "I noticed X, and it means Y" | `discovery` |
| "Here's how to do X" | `technique` |
| "Here's a way to think about X" | `framework` |
| "This happened, and I learned X" | `story` |
| "This thing exists and here's how to use it" | `tool` |

**When uncertain:** Default to `discovery`. It's the most flexible.

### Phase 2: Extract the Thesis

The thesis is the insight in one sentence. Not a summary. The teaching itself.

**Good thesis patterns:**
- "X works because Y" (explains mechanism)
- "When A happens, do B" (prescribes action)
- "The difference between X and Y is Z" (clarifies distinction)
- "X is really Y in disguise" (reveals hidden structure)

**Thesis anti-patterns:**
- Too vague: "Communication is important"
- Too long: More than one sentence
- No insight: Just describes, doesn't teach
- Obvious: Reader already knows this

**Invoke voice skill** if the thesis feels stiff. The thesis should sound like Robbie talking.

### Phase 3: Structure the Content

Use the type-specific template from `seeds/SCHEMA.md`.

**For discovery:**
1. What did you observe? (raw noticing)
2. What evidence supports it? (concrete examples)
3. What does it mean? (implications)

**For technique:**
1. What problem does this solve?
2. How do you do it?
3. Where does it apply?
4. What does success look like? (patterns)
5. What does failure look like? (antipatterns)

**For framework:**
1. What are the components?
2. How do they relate?
3. How do you use the framework?

**For story:**
1. What was the situation?
2. What happened? (events in sequence)
3. What did you learn?

**For tool:**
1. What does it do?
2. How do you use it?
3. Show examples

### Phase 4: Set Compilation Hints

- **audience:** Who needs this? (beginners, practitioners, architects)
- **tone:** How should it feel? (playful, serious, technical, reflective)
- **related_seeds:** What other seeds connect to this?
- **dependencies:** What must be understood first?

### Phase 5: Generate the Seed

Output valid YAML following the schema:

```yaml
name: insight-name-here
version: 1
status: raw

captured: YYYY-MM-DD
source_session: "session reference if known"

thesis: "One sentence insight goes here"

type: discovery | technique | framework | story | tool

content:
  # Type-specific fields here

compilation_hints:
  audience: practitioners
  tone: technical
  related_seeds: []
  dependencies: []

publication:
  compiled: false
  published: false
  published_at: null
```

**File naming:** `{name}.seed.yaml` — use the exact `name` value with `.seed.yaml` extension.

**Initial location:** `seeds/raw/` — all new seeds start here.

## Status Guidance

| Status | When to use |
|--------|-------------|
| `raw` | Just captured, may need refinement |
| `developing` | Actively being enriched, not yet complete |
| `ready` | Complete, validated, can be compiled |

**Don't overthink initial status.** Capture as `raw`, refine later.

## Quality Checks

Before finalizing:

- [ ] Name is kebab-case and descriptive
- [ ] Thesis is exactly one sentence
- [ ] Type matches the insight's nature
- [ ] Content fields are populated (can be brief for `raw`)
- [ ] Audience and tone are specified
- [ ] YAML syntax is valid

## Anti-Patterns

| Pattern | Problem | Fix |
|---------|---------|-----|
| Thesis as summary | Loses the teaching | Extract the actual insight |
| Over-structuring raw seeds | Blocks capture flow | Capture rough, refine later |
| Wrong type forcing | Awkward content fit | Let the insight choose its type |
| Skipping compilation hints | Compiler has no guidance | At minimum: audience + tone |
| Perfectionism | Never capture anything | Raw is good enough to start |

## After Capture

The seed exists. What next?

1. **Leave it raw:** It's preserved. Come back later.
2. **Develop it:** Move to `seeds/developing/`, enrich content.
3. **Mark ready:** When complete, move to `seeds/ready/`.
4. **Compile:** Invoke `seed-compiler` to generate article draft.

The seed garden grows. Articles emerge when seeds are ready.

---

*Capture first. Refine later. The insight is the seed.*
