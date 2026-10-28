# Generated audio provenance contract

This is a metadata contract, not approval to distribute any recording. The
playground still has no enabled synthesizer or bundled curated sound bank.

Write project-local `*.audio-provenance.json` files using
`web/wasm/play/audio-provenance.schema.json`. `kind` is
`infernal.audio.provenance`, `schema_version` is 1. Each source retains its own
stable ID, title, preferred ARTIST (AUTHOR fallback), source URL, license ID and
URL, optional exact attribution, modifications, and source kind. A phoneme is
an audio recording with its own license; an IPA symbol is not proof of permission.

Each bank/output declares **all** contributing `source_ids`, not just the most
recognizable one. Root `used_bank_ids` and `used_output_ids` select the entries
that must feed the game's credits. Empty/missing usage means an inactive catalog.
A declared reachable PCM voice bank can therefore feed credits without an Ogg
file existing. Runtime-created banks must register or persist their actual usage;
a static exporter cannot discover unrecorded runtime state.

Banks and outputs have `license_review: pending | approved | blocked`.
`approved` records an explicit compatibility review, not a conclusion computed
from a credit line. `blocked` entries must not generate distributable output.
`pending` remains visibly pending. Combining incompatible share-alike versions
is not made permissible by listing them. A resulting effective output license
is separate from, and never replaces, every underlying source's terms.

The Destructos adapter rejects duplicate/dangling IDs and malformed or oversized
records. It scans only project-local manifests, not URLs or arbitrary paths.
It appends source credits to canonical CREDITS.md-derived credits, stripping file
extensions only from displayed fallback names. It must preserve titles, artists,
license URLs, source URLs, exact required attribution and modifications.

For baked Vorbis output, write useful ARTIST/TITLE/LICENSE tags, plus this sidecar
for the complete source list and transformations. Do not attribute generated
work to the DSP library author or make a blanket CC0 claim. Credits generation
is attribution plumbing, not legal compatibility validation.
