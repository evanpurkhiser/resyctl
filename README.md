# resy

A Rust command-line client for Resy focused on automation-friendly, JSON-only output.
It is designed for agents and scripts to drive booking workflows and other
reservation automations.

## Install

From source:

```bash
cargo install --path .
```

## Configure auth

```bash
resy auth login --email "you@example.com" --password-file ./password
resy auth status
```

## Example Usage

```bash
# 1) Search for a restaurant (show 2 results)
resy search "ishq" --limit 2 \
  | jq -r '.venues[:2][] | "\(.id): \(.name) [\(.locality // "?")]"'

# Example output:
# 84214: Ishq [New York]
# 66703: Ishi Omakase & Premium Sake [New York]

# Pick the first venue id for the rest of this flow.
VENUE_ID=84214

# 2) Check availability for party size 2 on a specific date.
resy availability "$VENUE_ID" --date 2026-05-01 --party-size 2 \
  | jq -r '.slots[] | "\(.slot_id) | \(.start) | \(.type // "?")"'

# Save a slot id to quote/book.
SLOT_ID=$(resy availability "$VENUE_ID" --date 2026-05-01 --party-size 2 \
  | jq -r '.slots[0].slot_id')

# 3) Quote details for the slot (fee/cutoff/payment summary).
resy quote "$SLOT_ID" \
  | jq '{
      fee_amount: .quote.fee_amount,
      fee_display: .quote.fee_display,
      fee_cutoff: .quote.fee_cutoff,
      payment_type: .quote.payment_type,
      policy_text: .quote.policy_text
    }'

# 4) Book the slot.
# If this slot has a fee, pass --allow-fee (and optionally --max-fee / --max-cutoff-hours).
resy book "$SLOT_ID" --allow-fee --yes \
  | jq '{reservation_id, resy_token, book_token_expires, fee: .quote.fee_display}'

# 5) List upcoming reservations (show one just made + another earlier that day).
resy reservations --upcoming \
  | jq -r '.reservations[] | "\(.reservation_id) | \(.day) \(.time_slot) | \(.venue.name // "?")"'

# Isolate two reservations from the same day and choose the older one.
OLDER_TOKEN=$(resy reservations --upcoming \
  | jq -r '
      .reservations
      | map(select(.day == "2026-05-01"))
      | sort_by(.time_slot)
      | .[0].resy_token
    ')

# 6) Cancel the older reservation from earlier in the day.
resy cancel "$OLDER_TOKEN" --yes \
  | jq '{canceled, refund: .result.payment.transaction.refund}'
```

## Notes

- All command output is JSON.
- `resy book` enforces cancellation-fee guardrails by default.
- Use `resy payment-methods` to inspect available payment method IDs.
