# cron-FQlew — fingerprint manifest (velib-mcp slice)

```
job-id:           FQlew
oldest-issue-id:  one_track#2
fingerprint-date: 2026-05-13
prior-crons:      nnkMo (PR #142), FNrH6 (PR #165), xR5Hn (PR #168)
```

## Run scope

Cross-repo run covering the 15 oldest `author:DominicBurkart` open issues
across managed repos. This file is the **velib-mcp** slice (issue #9,
MCP documentation endpoint). Companion manifests live in `one_track`
and `nanna-coder` on branches `claude/<name>-FQlew`.

## Per-issue disposition

| issue | canonical PR | head sha   | mergeable | disposition |
|------:|-------------:|------------|-----------|-------------|
| #9    | **#142**     | `9751ac3f` | **clean** | NO-OP — defer to #142 |

## Stale head check (vs prior cron xR5Hn, 2026-05-10)

| PR  | xR5Hn recorded sha | current head sha (2026-05-13) | drift |
|----:|--------------------|-------------------------------|-------|
| #142 | `9751ac3f` | `9751ac3f` | none  |

No drift since xR5Hn. PR #142 remains `mergeable_state=clean` against
current `main` (`c2dc6ead`). This is exactly the "same trigger → same
no-op" branch of rule (1).

PR #114 (older alternative — `describe_api` shape) remains
`mergeable_state=dirty` with 10 unresolved owner threads. Both
cron-FNrH6 and cron-xR5Hn already recommended closing #114 as duplicate
of #142; that recommendation stands.

## Janitor actions (carried from prior crons)

- **Promote** PR #142 — it is the canonical implementation for #9 and
  is currently the only `mergeable=clean` PR among the in-scope issues
  across all three repos.
- **Close** PR #114 as duplicate of #142.

## Promote-to-human

Per CRON contract clause (3) — **planner jobs never promote**. This PR
is intentionally **NOT** labeled `ready-for-review`. The janitor job
decides promotion based on PR #142, not this manifest.

## Duplicate-detection guidance for janitor

This manifest PR carries the same job-id (`FQlew`) and oldest-issue-id
(`one_track-2`) labels as the companion manifests in `one_track` and
`nanna-coder`. Prior-cron manifest PR #168 (`xR5Hn`) covers the same
issue with identical disposition; on sight, both can be closed by the
janitor once PR #142 is promoted.

## Tags

- `agent-job:FQlew`
- `oldest:one_track-2`
