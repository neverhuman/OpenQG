# Paper Question Bank

This directory stores durable, reviewable JSON records for OpenQG real-paper
memory benchmark challenges.

Policy:

- Only open-access or public-license paper text may be checked in here.
- Do not check in raw PDFs, fetched HTML pages, provider payload logs, secrets,
  prompt-injection samples, or temporary extraction files.
- Raw receipts belong under `target/openqg/research/<loop>/latest/`.
- `papers/<publication_hash>.json` is the canonical full-text paper body record.
- `challenges/<challenge_hash>.json` stores accepted hard questions.
- `rejected/<challenge_hash>.json` stores rejected or ambiguous questions with
  the rejection reason.
- Never create a second paper body record with an existing `publication_hash`.

Validate with:

```sh
just research-validate
just research-dedupe-check
```
