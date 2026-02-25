---
title: "SOP Template"
category: "sop"
owner: "architecture-owner"
status: "active"
last_reviewed: "2026-02-25"
audience: ["engineering", "platform"]
invariants:
  - "All SOPs follow a consistent section order."
  - "SOPs preserve commands, expected outputs, and failure conditions."
tags: ["sop", "template"]
domain: "docs"
lifecycle: "ga"
---

# SOP Template

## 1. Title & Purpose

This SOP defines the procedure for `<procedure>` without violating `<critical reliability constraint>`.

## 2. Scope

- Covers: `<included systems/tasks>`
- Does not cover: `<explicit exclusions>`

## 3. Roles & Responsibilities

| Role | Responsibility |
| --- | --- |
| Operator | Executes the procedure |
| Reviewer | Verifies postconditions |
| Architect | Approves deviations |

## 4. Prerequisites

- `<tool/version>`
- `<credentials/access>`
- `<maintenance window or approvals>`
- `<data integrity check>`

## 5. Step-by-Step Procedure

1. `<step>`
   - Command:

   ```bash
   <command>
   ```

   - Expected output: `<what success looks like>`
   - Failure condition: `<what requires stop/escalation>`
2. `<step>`
   - Command:

   ```bash
   <command>
   ```

   - Expected output: `<what success looks like>`
   - Failure condition: `<what requires stop/escalation>`

## 6. Visual Aids

```mermaid
sequenceDiagram
  participant Operator
  participant System
  Operator->>System: <action>
  System-->>Operator: <result>
```

## 7. Invariants (Critical Section)

- `<non-negotiable safety rule>`
- `<consistency rule>`
- `<data integrity rule>`

## 8. Validation Checklist

- [ ] `<postcondition>`
- [ ] `<monitoring/log condition>`
- [ ] `<business/technical verification>`

## 9. Version History

| Version | Date | Author | Change |
| --- | --- | --- | --- |
| 0.1.0 | 2026-02-25 | `<author>` | Initial draft |

