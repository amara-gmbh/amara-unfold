# Independent local-app acceptance checklist

Use this checklist to review an amara Unfold candidate without relying on
private development records. The filename is retained for compatibility with
existing transfer-package scripts; the checklist is tool-neutral.

Use only public synthetic fixtures for a publishable result. Personal exports,
generated Markdown, absolute paths, screenshots containing personal data, and
local-library counts must stay private and must not enter source control.

## Candidate identity

Record:

- version and build number shown by the candidate;
- SHA-256 of the delivered archive or executable;
- source commit or source archive checksum;
- macOS version and hardware architecture;
- code-signing result, signer identity, and notarization status.

Do not describe an ad-hoc signature as Developer ID signing or notarization.

## Public release boundary

Confirm that the visible application offers only the documented application
routes: Apple Photos export, Calendar ICS, Contacts VCF, Apple Mail MBOX, and
Apple Music local/TSV input. Outlook PST and direct Immich-library intake must
not appear as selectable release routes.

## Visible checks

For every route exercised:

1. Select the synthetic source with the normal picker or visible route control.
2. Confirm recognition before any output is written.
3. Refuse consent and verify that no extraction begins.
4. Accept consent, select an empty output directory, and finish extraction.
5. Inspect the visible completion and quality summaries.
6. Confirm readable Markdown, schema-valid frontmatter, provenance, balanced
   accounting, and `networkUsed: false`.
7. Rerun without changing the input. Confirm that unchanged retained files and
   events are reported accurately and no duplicates appear.
8. Exercise cancellation where offered, confirm preservation of the last
   complete result, and then retry.

Use [Local-app intake testing](local-app-uat.md) for route-specific checks.

## Privacy checks

- No source or output leaves the local computer.
- No account, analytics, telemetry, or background synchronization is required.
- Apple Mail uses an exported `.mbox`; it does not require access to the live
  protected Mail store.
- Apple Photos uses an exported folder; it does not open the live Photos
  library.
- The public evidence contains no personal content, usernames, absolute paths,
  private counts, tokens, credentials, or local databases.

## Result template

```text
amara Unfold independent acceptance: PASS|FAIL|BLOCKED
Version/build:
Candidate SHA-256:
Source fingerprint:
Synthetic fixture:
Routes checked:
Recognition and consent:
Output and quality:
Rerun retention:
Cancellation and retry:
Network used: false|unexpected
Signing/notarization:
Blocking defects:
```

`PASS` means the recorded synthetic journeys and candidate-integrity checks
succeeded. It does not certify untested private export variants, Developer ID
signing, or notarization.
