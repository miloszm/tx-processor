# Transaction Processor

A small CLI that reads a CSV of transactions and outputs final client account states.

## Usage

```
cargo run -- input.csv
```

Output is written to stdout as CSV with columns: `client,available,held,total,locked`.

## Input format

```csv
type,client,tx,amount
deposit,1,100,10.00
withdrawal,1,101,3.50
dispute,1,100,
resolve,1,100,
chargeback,1,100,
```

- `type`: `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback`
- `client`: `u16` client ID
- `tx`: `u32` transaction ID
- `amount`: decimal, only required for `deposit` / `withdrawal`

## Model

Two state machines:

**Client balances** (`ClientData`):
- `available + held == total` always holds
- `held` is money tied up in open disputes
- `locked` is set permanently by a chargeback

**Transaction lifecycle** (`TxState`):

```
Active ──dispute──> Disputed ──resolve──> Resolved
                       │
                       └────chargeback──> ChargedBack
```

A `TransactionRecord` keeps the original op (`Deposit` or `Withdrawal`) and amount immutable; only `state` changes. This keeps `dispute`/`resolve`/`chargeback` symmetric and idempotent.

## Rules

| Op | Requires | Effect |
|---|---|---|
| `deposit` | account not locked | `available += amount`, `total += amount` |
| `withdrawal` | account not locked, `available >= amount` | `available -= amount`, `total -= amount` |
| `dispute` | tx exists, `state == Active`, same client, `available >= amount` | `available -= amount`, `held += amount`, `state = Disputed` |
| `resolve` | tx exists, `state == Disputed`, same client | `available += amount`, `held -= amount`, `state = Resolved` |
| `chargeback` | tx exists, `state == Disputed`, same client | `held -= amount`, `total -= amount`, `locked = true`, `state = ChargedBack` |

## Design decisions

**Business rejections vs errors.** `InsufficientFunds`, `AccountLocked`, etc. are returned as `Ok(OpOutcome::Rejected(..))`, not `Err`. `Err(TxProError)` is reserved for I/O and parse failures. This lets the driver log-and-continue on rejections while `?` still aborts on real errors.

**Locked accounts.** A locked account rejects `deposit` and `withdrawal` (external money movement), but not `dispute`/`resolve`/`chargeback`. Rationale: locking prevents new damage but allows open disputes to be adjudicated. The spec is silent here; this is the chosen interpretation.

**Transaction state vs type.** `TransactionRecord.type_op` never changes; `state` carries the lifecycle. This avoids the "was this a deposit or a dispute?" ambiguity that arises when a single field tries to encode both.

**No double-apply.** Every mutating op checks the tx state before mutating, so calling `dispute` twice, `resolve` twice, etc. is a no-op the second time.

**Ordered output.** `client_records` iterates a `BTreeMap`, so output is sorted by client ID — no post-sort needed.

## Error handling

- `TxProError::Io` — file couldn't be opened
- `TxProError::Csv` — malformed CSV
- `TxProError::MissingColumn` — header lacks a required field
- `TxProError::BadField` — a value couldn't be parsed (line + column in message)
- `TxProError::BadTypeOp` — unknown `type` value (line + value in message)

Parse errors report the physical line number and the offending column/value.

## Tests

`cargo test` covers each op's success path, each rejection reason, idempotency, wrong-client and locked-account guards, the `available + held == total` invariant, and a few end-to-end scenarios.

## Dependencies

- `csv` — streaming reader
- `rust_decimal` — fixed-precision decimal (4 dp)
- `thiserror` — error enum boilerplate
- `rust_decimal_macros` (dev) — `dec!()` literals in tests

## Notes
Amounts are accepted with 4 decimal points, extra decimal points if provided will be dropped.
