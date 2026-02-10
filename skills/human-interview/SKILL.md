---
name: human-interview
description: Low-friction interviewing for maximum signal extraction. Use when conducting any interview, intake, discovery session, or conversation where Claude needs to gather information from a human. Triggers include onboarding sessions, requirements gathering, project kickoffs, understanding user needs, debugging user problems, or any context where quality information from the human is the primary goal.
---

# Human Interview Skill

Conduct interviews that minimize cognitive burden on the human while maximizing signal-to-noise ratio.

## Core Principle

**Friction is the enemy of signal.** Every ounce of cognitive load you place on the human is energy they can't spend giving you quality information. Lower the coefficient of friction; the data flows.

## The Cardinal Rules

### 1. One Question Per Message

Humans can only process one question at a time. Multiple questions create:
- Cognitive overload → degraded responses
- Cherry-picking → they answer the easiest, skip the valuable
- Confusion → they lose track of what you asked

**Never**: "What's your goal, and what have you tried, and what's blocking you?"

**Always**: "What's the core outcome you're trying to achieve?"

### 2. Open-Ended by Default

Questions that can't be answered yes/no unlock narrative. Use stems:
- "Tell me about..."
- "Walk me through..."
- "How did you..."
- "What led you to..."
- "Describe..."

**Bad**: "Did you try restarting it?"
**Good**: "Walk me through what happened right before the error."

### 3. Follow the Energy

When something lights up—more words, more detail, more emotion—that's signal. Probe there:
- "Tell me more about that."
- "How so?"
- "What made that stand out?"
- "Say more about [specific phrase they used]."

### 4. Comfortable Silence

After asking, wait. Don't rush to fill silence. The human needs processing time. Silence is not awkward—it's productive.

## Interview Flow Pattern

### Phase 1: Warm-Up (1-2 exchanges)
Establish rapport. Low-stakes, easy questions.
- "What brought you here today?"
- "Give me a quick sense of what you're working on."

Purpose: Lower activation energy. Make them feel heard before you dig.

### Phase 2: Exploration (bulk of session)
Open-ended discovery. Let them lead the narrative.
- "Tell me about [the thing]."
- Follow threads that emerge.
- Use neutral facilitators: "I see", "Go on", "Mm-hmm" (in text: "Got it", "I'm following", "Makes sense").

### Phase 3: Clarification
Fill gaps with targeted follow-ups.
- "You mentioned X—can you expand on that?"
- "What happened between A and B?"
- "Help me understand the connection between..."

### Phase 4: Confirmation
Reflect understanding back.
- "So if I'm hearing you right..."
- "Let me make sure I've got this..."

This catches misunderstandings AND signals you were listening.

## Probing Techniques

### Laddering (Broad → Specific)
Start wide, narrow based on responses.
```
"Tell me about your morning routine."
↓
"You mentioned the email overwhelm—what does that look like?"
↓
"When you say 'too many threads'—give me a number."
```

### The Five Whys
Keep asking why until you hit bedrock.
```
"Why did you choose that approach?"
"Why was speed the priority?"
"Why would delay have been costly?"
```
Stop when they can't explain further—you've hit a core value or constraint.

### Echo Technique
Repeat their exact words as a question.
- Human: "It just felt off."
- Claude: "'Felt off'?"
- Human: [elaborates with 10x more detail]

### Hypothetical Reframe
When stuck, shift perspective.
- "If you had unlimited resources, what would you do?"
- "If this problem didn't exist, what would you be doing instead?"

## Anti-Patterns to Avoid

| Anti-Pattern | Why It Kills Signal |
|--------------|---------------------|
| Multiple questions | Cognitive overload, partial answers |
| Leading questions | You get your assumptions back, not truth |
| Yes/no questions | Binary answers have no texture |
| Interrupting | Breaks their flow, signals you're not listening |
| Jargon they don't use | Creates distance, requires translation |
| Assuming shared context | Gaps go unspoken |
| Repeating questions | Signals you weren't listening; fatigues them |
| Marathon sessions | Diminishing returns after ~45min |

## Reducing Participant Fatigue

- **Set expectations early**: "This should take about 10-15 minutes."
- **Show progress**: "Just a couple more areas to cover."
- **Validate their input**: "That's really helpful" (genuinely, not reflexively).
- **Don't repeat yourself**: Track what's been covered.
- **Watch for fatigue signals**: Shorter answers, "I don't know" increases, restlessness.

## Special Situations

### When They're Vague
Don't accept "it's complicated." Probe gently.
- "Give me one specific example."
- "If you had to pick the biggest piece of it..."

### When They're Overwhelmed
Slow down. Simplify.
- "Let's focus on just one thing. What's the first thing that comes to mind?"

### When They're Defensive
Soften. Remove judgment.
- "There's no wrong answer here—I'm just trying to understand."
- Shift to third person: "How do people usually handle this?"

### When You Need Specifics
Make it concrete.
- "Walk me through the last time this happened."
- "Show me / describe what you see on screen."

## The Meta-Skill

**Genuine curiosity is the unlock.** If you're authentically interested in understanding their world, the right questions emerge naturally. The techniques above are training wheels for curiosity—internalize the stance, and the tactics follow.

## Quick Reference

```
START:    Warm-up → one easy open-ended question
EXPLORE:  "Tell me about..." → follow energy → probe depth  
CLARIFY:  "You mentioned X..." → fill gaps
CONFIRM:  "Let me make sure I understand..."
CLOSE:    "Anything else I should know?"
```

Remember: **One question. Open-ended. Follow the energy. Wait.**
