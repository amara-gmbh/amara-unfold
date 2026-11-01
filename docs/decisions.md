# Design and decision record

This is the curated public rationale for the choices that materially shape
Amara Unfold. It is not a transcript. Decisions affecting public behavior must
be understandable here, in the changelog, in a public contract, or in source
and tests without requiring access to private development conversations.

## D001 — Local-first is the default trust boundary

**Decision:** Source data is processed on the user's computer. The current
runtime denies outbound network intent and includes no telemetry, analytics,
background synchronization, account requirement, or cloud extraction service.

**Why:** Personal exports can contain an unusually complete picture of a life.
Local processing reduces unnecessary disclosure and lets users inspect the
software's behavior.

**Consequences:** A future network feature requires a separate review,
explicit user action, narrow destination, visible disclosure, and updated tests
and documentation. See [Privacy model](privacy.md).

## D002 — The user chooses the source boundary

**Decision:** Unfold opens only an explicitly selected file, folder, or
supported application export route. Recursive scanning does not follow
symbolic links.

**Why:** Convenience must not become silent filesystem surveillance or cross a
boundary the user did not select.

**Consequences:** Application routes may require a prepared export rather than
direct access to private application databases.

## D003 — Markdown is the primary durable output

**Decision:** Human-readable Markdown is the canonical user-facing export.
Small YAML/CSV indexes may support deterministic validation or navigation.

**Why:** The output should remain readable, searchable, editable, portable, and
useful without Amara software.

**Consequences:** Machine-oriented files must justify their existence. The
event contract is public in [Open Event Schema v1](schema-v1.md).

## D004 — Complete catalogues and Timeline events are different products

**Decision:** Preserve complete useful source metadata in catalogues, but add
only defensible activity to the Timeline.

**Why:** A metadata catalogue should be complete; a life Timeline should not be
flooded by static reference rows, duplicate representations, or operational
noise.

**Consequences:** Apple Music, for example, preserves tracks, playlists, and
membership in catalogues while Timeline uses explicit track `Date Added` only.
Timeless reference notes remain available without an invented date.

## D005 — Dates and identities require evidence

**Decision:** Never invent a date, person, place, relationship, or causal
connection. Preserve date precision instead of manufacturing an exact day.

**Why:** A polished falsehood is worse than an honest unknown in a personal
history.

**Consequences:** Unknown, year, month, day, and instant precision remain
distinct. Cross-source connections are reviewable evidence candidates, not
claims about companionship or causation.

## D006 — Manual dating is explicit, auditable, and reversible

**Decision:** A user may manually place a genuinely undated event using a date
they can verify. The correction records a revision and supports correction and
undo.

**Why:** Source exports can omit usable dates even when the user knows them.
User knowledge should be addable without rewriting raw evidence or pretending
the source supplied it.

**Consequences:** Stable ID, original Markdown, provenance, raw path/hash, and
the original unknown state remain preserved. Source-dated events and timeless
reference notes are protected from this workflow.

## D007 — One adapter implementation serves app and CLI

**Decision:** Native UI and command-line extraction call the same Rust adapter
library.

**Why:** Two implementations would drift in parsing, normalization, skip
accounting, filenames, and Timeline behavior.

**Consequences:** Adapter changes need one source implementation plus unit,
end-to-end, browser/native, and route-specific acceptance where applicable.

## D008 — Recognition precedes extraction

**Decision:** The application names the detected source and format before
asking for an output destination or creating Markdown.

**Why:** Users should understand what Unfold believes it found and be able to
stop a wrong route before files are written.

**Consequences:** Recognition must generalize across real export variants and
cannot depend on one person's filename.

## D009 — SQLite is a rebuildable local index, not the archive

**Decision:** Bundle SQLite for fast local state, but keep raw exports and
generated Markdown outside the database as the durable evidence and output.

**Why:** The app needs transactions, cancellation safety, and queryable state
without turning a private opaque database into the user's only copy.

**Consequences:** A completed rescan atomically replaces a source snapshot. A
cancelled scan preserves the last complete snapshot. Layer removal is
index-only.

## D010 — Tauri, Rust, and React have separate responsibilities

**Decision:** Rust owns filesystem, extraction, validation, database, and
native IPC behavior. React owns presentation and interaction through a typed
bridge. Tauri supplies the narrow native shell and capabilities.

**Why:** Personal-data access and evidence rules benefit from typed native code,
while the interface benefits from a composable web renderer.

**Consequences:** The renderer cannot directly roam the filesystem. Native
capabilities and CSP remain explicit configuration.

## D011 — Availability is proved, not inferred from code

**Decision:** A route appears under **Available now** only after its complete
end-user path passes installed-application acceptance. Parser code alone is not
a product promise.

**Why:** Recognition, permissions, source preparation, extraction, output,
quality, and UI readability can fail independently.

**Consequences:** Incomplete routes remain under **Not available yet** or in
technical adapter documentation without an enabled user action.

## D012 — Quality accounting must balance

**Decision:** Every adapter reports records seen, events written, skipped
records with named reasons, extraction errors, duplicates, and network use.

**Why:** A success message without accounting can conceal missing or misleading
output.

**Consequences:** Quality percentages supplement rather than override material
review findings. See [Data quality and validation](quality-and-validation.md).

## D013 — Browser checks do not replace installed-app UAT

**Decision:** Browser scenarios are intermediate and deterministic. Final
production acceptance uses the rebuilt native application through real macOS
permissions, filesystem access, IPC, and output review.

**Why:** A WebView preview cannot prove native pickers, permissions, SQLite,
application automation, bundle resources, or installed behavior.

**Consequences:** Both layers remain valuable and have separate tests. See
[Testing](testing.md).

## D014 — Public source and private evidence remain separate

**Decision:** Publish buildable application source, public documentation,
reviewed assets, configuration, scripts, tests, and lockfiles. Exclude personal
corpora, raw takeouts, runtime databases, generated exports, private working
memory, local paths, secrets, build output, and signing credentials.

**Why:** Open source should make the product independently understandable and
buildable without publishing a user's life or internal recovery material.

**Consequences:** `.gitignore` is only the first gate. Content and reachable
history audits must also pass. See [Trust and audit](trust-and-audit.md).

## D015 — Unfold remains useful without Amara Memory

**Decision:** Unfold is the open preparation layer for optional Amara Memory,
but its files and Timeline must remain independently useful.

**Why:** A path into a richer experience should not recreate platform lock-in.

**Consequences:** No Amara account is required, no live bridge is claimed in
the current build, and another local tool may read the open files.

## D016 — Public source is allowlisted and history is independently gated

**Decision:** Generate public source only from a reviewed top-level allowlist,
remove canonical-only continuity tooling and commands, and audit the resulting
tree. Validate reachable Git authors, committers and annotated taggers
separately against the exact company identity.

**Why:** File exclusions alone fail open when a new root path appears, and a
privacy-safe working tree does not remove private content or personal identity
from a previously used Git object database.

**Consequences:** New public root paths require explicit review. Boundary
regressions must prove that private tooling, commands and unapproved roots are
rejected. A replacement publication history starts in a never-imported object
database; publication remains separate from source preparation.
