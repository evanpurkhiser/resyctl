# ressy CLI Spec (Iteration 0)

This document defines the initial command-line interface for `ressy`, a streamlined CLI for restaurant availability discovery and safe reservation booking.

## Goals

- Keep the interface fast and simple for interactive use.
- Make automation reliable with stable JSON output and exit codes.
- Default to safe booking behavior around cancellation fee policies.

## Command Name

- Primary command: `ressy`
- Optional alias: `reggie`

## Design Principles

- One core workflow: discover slots, then book a slot by `slot_id`.
- `book` should not require raw time/location/seating parsing.
- Every command should support machine-readable output via `--json`.
- Human output should be concise and scannable.

## Core Workflow

1. Search for a restaurant/venue.
2. Check availability (days and then times).
3. Inspect cancellation and fee policy.
4. Book a selected slot.

## Commands (v0.1)

### `ressy search <query>`

Search for restaurants/venues.

Examples:

```bash
ressy search "lilia"
ressy search "via carota" --city nyc --json
```

Proposed flags:

- `--city <name>`
- `--limit <n>`
- `--json`

### `ressy availability`

List available days and times for a restaurant.

Examples:

```bash
ressy availability <restaurant_id> --month 2026-05 --days
ressy availability <restaurant_id> --date 2026-05-12
ressy availability <restaurant_id> --date 2026-05-12 --time-after 18:00 --time-before 21:00
```

Proposed flags:

- `<restaurant_id>` (required positional argument)
- `--month YYYY-MM`
- `--days` (show only days with any availability)
- `--date YYYY-MM-DD`
- `--party-size <n>`
- `--seating <bar|table|patio|any>`
- `--location <id|name>`
- `--time-after HH:MM`
- `--time-before HH:MM`
- `--json`

Return data should include, per slot:

- `slot_id`
- `datetime_local`
- `seating`
- `location`
- `cancellation_policy`
- `fee_amount`
- `fee_currency`
- `fee_cutoff_datetime`
- `bookable` (under current safety rules)

### `ressy quote <slot_id>`

Show booking and cancellation policy details for a slot.

Examples:

```bash
ressy quote <slot_id>
ressy quote <slot_id> --json
```

Proposed flags:

- `--json`

### `ressy book <slot_id>`

Book a reservation from a specific slot.

Safe defaults:

- Block non-cancelable or fee-bearing bookings unless explicitly allowed.

Examples:

```bash
ressy book <slot_id>
ressy book <slot_id> --allow-fee --max-fee 25 --max-cutoff-hours 12 --yes
ressy book <slot_id> --dry-run
```

Proposed flags:

- `--allow-fee`
- `--max-fee <amount>`
- `--max-cutoff-hours <n>`
- `--yes`
- `--dry-run`
- `--idempotency-key <key>`
- `--json`

### `ressy reservations`

List existing reservations for sanity checks and automation guardrails.

Examples:

```bash
ressy reservations
ressy reservations --upcoming --json
```

Proposed flags:

- `--upcoming`
- `--past`
- `--limit <n>`
- `--json`

### `ressy auth`

Manage authentication/session information.

Examples:

```bash
ressy auth login
ressy auth status
ressy auth logout
```

### `ressy config`

Manage user defaults and automation policy thresholds.

Examples:

```bash
ressy config set default_party_size 2
ressy config set max_fee 20
ressy config get max_fee
ressy config list
```

## Global Flags

- `--json`
- `--quiet`
- `--verbose`
- `--profile <name>` (optional future)

## Exit Codes (proposed)

- `0`: success
- `2`: no availability found
- `3`: blocked by safety policy (fee/cancellation rules)
- `4`: API/auth/network error
- `5`: invalid CLI usage or validation error

## Automation Notes

- JSON response shape should be stable and versioned.
- `book` should support idempotency to avoid duplicate bookings.
- Timestamps should be explicit and include timezone offsets.

## Out of Scope (for now)

- Waitlist management
- Interactive TUI mode
- Multi-account/team usage

## Open Questions

- How exactly does the upstream API model locations and seating types?
- Can cancellation fees be detected and normalized before booking?
- Is there a first-class "quote" endpoint, or is quote data only in availability responses?
- What auth/session flow is supported for automation use (token vs cookie)?

## Next Iteration

- Add a concrete JSON schema for each command.
- Add exact `--help` text and argument validation rules.
- Map this spec to an implementation plan and command skeleton.
