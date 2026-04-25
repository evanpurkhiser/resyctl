# resyctl

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
resyctl auth login --email "you@example.com" --password-file ./password
resyctl auth status
```

## Example Usage

```bash
# 1) Search for a restaurant.
resyctl search "ishq" --limit 2 \
  | jq -r '.venues[:2][] | "\(.id): \(.name) [\(.locality // "?")]"'

# 84214: Ishq [New York]
# 66703: Ishi Omakase & Premium Sake [New York]

# Use the first result for the rest of the flow.
VENUE_ID=84214

# 2) Check availability for party size 2 on a specific date.
resyctl availability "$VENUE_ID" --date 2026-05-23 --party-size 2 \
  | jq -r '.slots[:4][] | "\(.slot_id[0:12])[…] | \(.start) | \(.type // "?")"'

# eyJjb25maWdfa[…] | 2026-05-23 12:15:00 | Bar Seat
# eyJjb25maWdfa[…] | 2026-05-23 12:15:00 | Dining Room
# eyJjb25maWdfa[…] | 2026-05-23 12:30:00 | Bar Seat
# eyJjb25maWdfa[…] | 2026-05-23 12:30:00 | Dining Room

# Save a slot id to quote/book.
SLOT_ID=$(resyctl availability "$VENUE_ID" --date 2026-05-23 --party-size 2 \
  | jq -r '.slots[] | select(.start=="2026-05-23 13:30:00" and .type=="Dining Room") | .slot_id' \
  | head -n1)
echo "${SLOT_ID:0:12}[…]"

# eyJjb25maWdfa[…]

# 3) Quote details for the slot (fee/cutoff/payment summary).
resyctl quote "$SLOT_ID" \
  | jq '{
      fee_amount: .quote.fee_amount,
      fee_display: .quote.fee_display,
      fee_cutoff: .quote.fee_cutoff,
      payment_type: .quote.payment_type
    }'

# {
#   "fee_amount": 25,
#   "fee_display": "$25.00",
#   "fee_cutoff": "2026-05-22T17:30:00Z",
#   "payment_type": "free"
# }

# 4) Book the slot.
# If this slot has a fee, pass --allow-fee (and optionally --max-fee / --max-cutoff-hours).
resyctl book "$SLOT_ID" --allow-fee --yes \
  | jq -r '"reservation=\(.reservation_id) token=\(.resy_token[0:12])[…] fee=\(.quote.fee_display)"'

# reservation=867457046 token=Ys7435rTmPAu[…] fee=$25.00

# 5) List upcoming reservations.
resyctl reservations --upcoming \
  | jq -r '.reservations
    | sort_by(.day, .time_slot)
    | .[:2]
    | .[]
    | "\(.reservation_id) | \(.day) \(.time_slot) | \(.venue.name // "?") | \(.resy_token[0:12])[…]"'

# 867250480 | 2026-04-29 18:00:00 | MOKYO | 4hRnr95|mdVS[…]
# 867248247 | 2026-05-01 18:30:00 | Antidote | A1xdLzgBrOOT[…]

# 6) Cancel the older upcoming reservation.
CANCEL_TOKEN=$(resyctl reservations --upcoming \
  | jq -r '.reservations | sort_by(.day, .time_slot) | .[0].resy_token')
resyctl cancel "$CANCEL_TOKEN" --yes \
  | jq '{canceled, refund: .result.payment.transaction.refund}'

# {
#   "canceled": true,
#   "refund": 1
# }
```

## Notes

- All command output is JSON.
- `resyctl book` enforces cancellation-fee guardrails by default.
- Use `resyctl payment-methods` to inspect available payment method IDs.
