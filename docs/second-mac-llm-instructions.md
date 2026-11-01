# Instructions for the LLM operating Amara Unfold on another Mac

## Role

You are operating a transferred Amara Unfold binary package. Your job is to
verify it, explain it, launch it when requested, and help the user convert an
explicitly selected personal-data export into open Markdown without changing
the source data.

Do not treat this package as a cloud product or as Amara Memory itself. Amara
Memory is only a possible downstream consumer of the exported Markdown.

## Read order

Before operating the package, read these local files in order:

1. `README-FIRST.md`
2. `manifest.json`
3. `BUILD-INFO.txt`
4. `CHECKSUMS-SHA256.txt`
5. `COMPONENTS.md`
6. `LOCAL-APP-UAT.md`
7. `LOCAL-APP-UAT-STATUS.txt`
8. `LOCAL-APP-UAT-STATUS.json`
9. `CLAUDE-LOCAL-APP-APPROVAL.md`
10. `PRIVACY.md`
11. [`open-adapters.md`](open-adapters.md)
12. `HOW-TO-USE.md`

Do not invent capabilities that are absent from those files.

## Initial gate

Run and report the important results from:

```sh
pwd
uname -m
sw_vers
shasum -a 256 -c CHECKSUMS-SHA256.txt
file amara-unfold
```

Proceed only when:

- the current directory is the transfer package;
- the destination Mac is `arm64`;
- every checksum is valid;
- `amara-unfold` is an `arm64` Mach-O executable.

If any condition fails, stop and report the mismatch. Do not download a
replacement or disable macOS security without the user's instruction.

## Safe launch procedure

```sh
ditto -x -k "Amara Unfold.app.zip" .
chmod +x amara-unfold
./amara-unfold adapters
open "Amara Unfold.app"
```

The build is unsigned. Prefer the normal Control-click **Open** flow. Remove
`com.apple.quarantine` only after checksum verification and only when macOS is
blocking this verified local package.

After launch, verify that the process actually exists. Do not claim the app is
running solely because `open` returned successfully.

## Operating rules

- Never scan the whole Mac automatically.
- Operate only on a source folder the user explicitly identifies.
- Treat source exports as immutable evidence. Never rename, move, rewrite, or
  delete their files.
- Use a separate destination folder for generated Markdown.
- Never upload archive data or use a network service to interpret it.
- Never copy personal data into the transfer package or source repository.
- Never delete the local SQLite index without explicit user approval.
- Never infer a person's identity, location, relationship, or activity without
  source evidence.
- Same-day records are context candidates, not proof that events are related.
- Preserve unresolved person and place IDs until evidence or the user resolves
  them.
- Report partial failures and quality warnings plainly.

## Preferred app workflow

1. Open Amara Unfold.
2. Choose **Choose a folder**.
3. Let Unfold inspect only that selected folder.
4. Review the recognized adapter routes with the user.
5. Choose a separate output folder.
6. Start the selected route or all recognized structured routes.
7. Keep the app open until extraction and timeline generation finish.
8. Inspect the package manifest and timeline quality report.
9. Report counts, errors, undated records, and unresolved entities.

Event filenames include readable titles, but the cross-source human overview
still starts with:

```text
<output>/Amara-Unfold-Timeline/index.md
<output>/Amara-Unfold-Timeline/quality-report.md
<output>/Amara-Unfold-Timeline/connections.md
```

## CLI workflow

List available adapter IDs:

```sh
./amara-unfold adapters
```

Extract one supported source:

```sh
./amara-unfold extract <adapter-id> --source <raw-path> --output <destination>
```

Build or rebuild the cross-source timeline after one or more extractions:

```sh
./amara-unfold timeline <destination>
```

Validate one generated service package:

```sh
./amara-unfold validate <package-folder>
```

Quote every path containing spaces. Do not guess an adapter ID; obtain it from
`./amara-unfold adapters` and confirm the raw dataset matches the contract in
[`open-adapters.md`](open-adapters.md).

## Expected output contract

Each service package contains:

```text
README.md
manifest.yaml
events/YYYY/MM/*.md
```

Each event is schema-v1 Markdown with YAML frontmatter, a human-readable body,
deterministic ID, source path, SHA-256 provenance hash, and no copied original.

The destination-level `Amara-Unfold-Timeline` contains day/month/year indexes,
an undated register, `catalog.csv`, entity indexes, Markdown/JSON connections,
and Markdown/JSON quality reports. It is derived and rebuildable. The linked
event Markdown remains the evidence source.

## Troubleshooting order

1. Confirm architecture and checksums.
2. Confirm the source and destination paths exist and are readable/writable.
3. Confirm the source is unpacked when the relevant adapter requires a folder.
4. Read the app's visible error or the CLI's standard error output.
5. Inspect `manifest.yaml` and `quality-report.md`; do not inspect private
   content more broadly than the user requested.
6. Retry with a small copied or synthetic fixture.
7. If a code change is required, state that the source tree is needed.

Do not install Node.js, Rust, Homebrew packages, a database server, or an LLM
runtime merely to run the supplied binaries. Those are development concerns,
not runtime dependencies.

## Binary-only limitation

When only this package is present, you may run, verify, and diagnose the
current binary behavior. You cannot responsibly patch or rebuild it. Ask for
the matching source archive or repository checkout before attempting code
changes.

## Success report

At completion, report:

- app and CLI build identity;
- destination Mac architecture and macOS version;
- checksum result;
- selected source and output paths;
- adapters run;
- records read, Markdown events written, unchanged events, and errors;
- timeline event/day/entity/cross-source counts;
- quality-report status and unresolved limitations;
- explicit confirmation that source files were not modified.

For the local-app Scan goal, use `CLAUDE-LOCAL-APP-APPROVAL.md` as the final
approval format. Do not approve while Apple Mail remains
`waiting_for_exported_mbox`.
