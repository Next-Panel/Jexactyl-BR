---
name: i18n-pipeline
description: Operates the Jexactyl translation queue, claims units, enforces the 0-10 review loop and records progress in SQLite via jx i18n-queue. Use for any translation category, page, label, review, correction or progress/resume request.
---

# i18n pipeline

Use this skill when translating the Jexactyl panel or operating its queue.

## Read first

1. `AGENTS.md`
2. `i18n/POLICY.md`
3. `i18n/glossary.tsv`
4. The category scope: globs in `i18n/catalog.json` plus resolved file lists per unit in `i18n/catalog.lock.json`
5. The queue state (`jx i18n-queue status`; `CACHE.md` is frozen history only)

## Start or resume

```bash
jx i18n-queue recover
jx i18n-queue status
jx i18n-queue next --role analyst --limit 3
```

## One stage at a time

- Claim at most 3 units in the same stage, never more than 3 active claims.
- Spawn one subagent per claimed unit, in parallel.
- The analyst and reviewer are read-only; the translator and fixer edit only
  the unit files.
- Reviewers must be independent from translators and fixers.

## Record

```bash
jx i18n-queue complete <unit> --role analyst --owner NAME --report <path> --labels-total N
jx i18n-queue complete <unit> --role translator --owner NAME --report <path> --labels-translated N
jx i18n-queue complete <unit> --role reviewer --owner NAME --score N --report <path>
jx i18n-queue complete <unit> --role fixer --owner NAME --report <path>
jx i18n-queue advance <category>
jx i18n-queue record-consistency <category> --score N --report <path>
jx i18n-queue record-gate <category> --name <gate> --status pass
jx i18n-queue record-patch <category> --name <patch> --unit <unit>
jx i18n-queue record-overlay <category> --path <path> --unit <unit>
```

## Rules

- Never translate code comments. Translate labels only.
- Score 10 means zero findings. Do not round up.
- The third review below 10 blocks the unit and category automatically.
- If the same blocker persists after the corrections, run `jx i18n-queue block` and
  escalate; escalation is a separate step from the automatic block.
- Use append-only patches: one batch per stage/round; no `--refresh` mid-loop.
- Only one category is active at a time.
