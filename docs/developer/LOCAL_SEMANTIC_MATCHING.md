# Optional Local Semantic Matching

JobSentinel has an optional `embedded-ml` Cargo feature for local semantic
matching. It is not required for core workflows, and it is separate from
external AI.

## Status

| Area | Current state |
| ---- | ------------- |
| Feature flag | `embedded-ml` |
| Default app behavior | Disabled unless built with the feature |
| Core workflow dependency | None; deterministic matching remains available |
| Data flow | Resume and job-skill matching runs locally |
| Model governance | `crates/jobsentinel-local-ai/models.lock.toml` pins model identity, revision, hashes, size, license, wired backend, instruction profiles, and score thresholds |
| Target embedding profile | `Qwen/Qwen3-Embedding-0.6B` at revision `97b0c614be4d77ee51c0cef4e5f07c00f9eb65b3`, 768-dimensional balanced profile |
| Target reranker profile | `Qwen/Qwen3-Reranker-0.6B` at revision `e61197ed45024b0ed8a2d74b80b4d909f1255473` |
| Current wired runtime | Direct matcher calls prefer the governed Qwen3 embedding plus reranker pair when both models are downloaded and checksum-verified, then verified `sentence-transformers/all-MiniLM-L6-v2`, then exact-only deterministic matching without a model download; embedded-ML resume/job scoring uses the hybrid scorer |
| Network behavior | Model download only, when the model is explicitly requested |
| User data sent during model download | None |
| Integrity check | Required SHA-256 checks for every required file in `crates/jobsentinel-local-ai/models.lock.toml` |
| Cache layout | `<app-data>/ml_models/<model-id>/<revision>/<model-lock-hash>/` |
| Runtime stack | Candle, tokenizers, `safetensors`, optional macOS Metal acceleration |
| Hybrid ranking | Typed local scoring core combines dense, BM25, exact skill, required-coverage, seniority, reranker, blocker, and provenance signals |

Focused Qwen3 embedding evidence: on 2026-06-19,
`core::ml::qwen3::tests::qwen3_backend_embeds_with_pinned_downloaded_model`
downloaded the pinned Qwen3 embedding model into an external test cache,
verified checksums, loaded the `qwen3-candle` backend, and returned a
normalized 768-dimensional embedding.

Focused Qwen3 reranker evidence: on 2026-06-19,
`core::ml::qwen3::tests::qwen3_reranker_ranks_direct_evidence_above_near_miss`
downloaded the pinned Qwen3 reranker model into an external test cache,
verified checksums, loaded the `qwen3-reranker-candle` backend, and ranked
direct Kubernetes security evidence above a vocabulary-overlap near miss.

Product integration evidence: the Settings **Local Match Check** panel calls
`get_semantic_matching_diagnostics`. Normal builds report the built-in local
fallback. `embedded-ml` builds report the checked-in Qwen3 model lock, required
file presence, cache readiness, scoring signals, local-only privacy mode, and
quality checks without loading model weights or exposing resume/job text.
Model Doctor reports absent default caches as needing a download. Any mixed,
partial, unreadable, symlinked, non-file, wrong-size, or checksum-invalid cache
reports a misconfiguration with a pinned re-download action. Inspection
requires regular files with exact pinned sizes before hashing. Diagnostics
expose model lock metadata and safe required-file counts, never cache paths or
file contents. When Qwen3 needs attention but verified MiniLM is ready, the
diagnostic names MiniLM as the active local matcher instead of claiming the
exact-only fallback is active.
An integrity-invalid default Qwen3 cache offers a native-reviewed removal
action. The backend accepts only an exact embedding or reranker ID from the
checked-in lock, rechecks the cache after confirmation, rejects missing,
incomplete, ready, legacy, unknown, and path-like requests, and never downloads
replacement files automatically. Cache paths and file contents remain private.
In `embedded-ml` builds, the same panel also offers governed setup and full
Qwen3 cache removal. Setup requires native confirmation that names the download
host, approximate size, license, local-only data boundary, and built-in
fallback, including notice that setup may need additional temporary disk
space. The Hugging Face client pins `https://huggingface.co`, disables implicit
ambient tokens before runtime startup, uses a fixed JobSentinel user agent, and
reports byte progress without exposing filenames to the renderer. Required
files stream directly into validated, app-owned partial files instead of the
Hugging Face cache layout. Dropping the stream cancels Xet-backed transfers;
verified files and the paired partial files remain available for retry. The
Xet transport cache is pinned under JobSentinel app data, reset before each
transfer, and cleared after success or explicit removal. Retry skips files that
already match the lock. Full removal requires native confirmation and deletes
all locally cached data under the two governed Qwen3 model identities,
including stale revisions, superseded lock layouts, and incomplete app-owned
staging and transfer data. Removal refuses to proceed if any entry inside the
governed tree is a symlink. On Windows, removal fails closed while model files
are in use by an active match and succeeds on retry. Built-in matching remains
available throughout.
Direct matcher calls now use Qwen3 dense retrieval plus bounded Qwen3
reranking when the governed model pair is present, fall back to verified MiniLM
when it is present, and use exact-only deterministic matching when no verified
model is available. The deterministic path does not claim related terms are
equivalent and does not download or write model files.

Gate 4 keeps the runtime matrix narrower than the model research. Deterministic
matching is the model-free default. The optional stronger path is the exact
checksum-verified Qwen3 Candle embedding and reranker pair. Verified MiniLM is
a legacy fallback only. FastEmbed, local HTTP, Python sidecars, OS-native
helpers, local LLM servers, and external AI are not approved matching
providers. The model lock therefore declares no alternative compatible backend
for either Qwen3 model.

Focused hybrid ranking evidence: `core::ml::hybrid` tests prove the ranking
core prefers direct evidence over keyword-only near misses, caps otherwise
strong matches when hard blockers exist, and records dense, BM25, exact skill,
required-coverage, and reranker provenance without raw resume text.
`core::resume::matcher::hybrid_score` wires the hybrid scorer into resume/job
match scores for `embedded-ml` builds while preserving the legacy weighted
formula when local ML is disabled.

Privacy label: **Local only** for matching. Model download is an explicit
external file fetch and must not send resume text, salary floors, notes,
application history, or job-search records.

## Product contract

- Local semantic matching is assistive and advisory.
- It may help compare a user's skills with visible job requirements.
- It must not replace plain, inspectable match explanations.
- It must not claim to predict hiring outcomes.
- It must fall back to deterministic matching when unavailable.
- Downloaded model files must stay revision-pinned and checksum-verified before
  cache status or loading reports success.
- Runtime compatibility must validate model id, backend, dimensions, tokenizer,
  pooling, normalization, max tokens, and instruction support before indexing or
  scoring.
- Vector provenance must include model id, repo, revision, backend, dimension,
  instruction profile, chunker version, normalizer version, pooling,
  normalization, and model-lock hash.
- A vector is stale when text, model revision, dimension, instruction profile,
  chunker version, normalizer version, pooling, or normalization changes.
- If exposed in user-facing UI, model download must have clear consent,
  progress, retry, cancel, and plain fallback copy.

Do not confuse this feature with external AI. External AI remains optional,
disabled by default, and routed through
[the privacy-first AI gateway](../security/privacy-first-ai-gateway.md).

## Code map

| Path | Purpose |
| ---- | ------- |
| `crates/jobsentinel-local-ai/src/mod.rs` | Feature-gated module entry and error types |
| `crates/jobsentinel-local-ai/src/contracts.rs` | Typed contracts for role families, generated-advice separation, skill graph relationships, evidence explanations, adversarial posting signals, and modular matching stages |
| `crates/jobsentinel-local-ai/src/manifest.rs` | Checked-in model lock parsing and validation |
| `crates/jobsentinel-local-ai/src/model.rs` | Thin model management entrypoint |
| `crates/jobsentinel-local-ai/src/model/` | Cache verification, download, metadata, loading, device selection, integrity checks, and legacy baseline inference |
| `crates/jobsentinel-local-ai/src/qwen3.rs` | Thin Qwen3 module entry |
| `crates/jobsentinel-local-ai/src/qwen3/` | Governed Qwen3 embedding, reranker, model, pooling, and tokenization implementation |
| `crates/jobsentinel-local-ai/src/runtime.rs` | Backend traits, runtime compatibility, vector provenance, and stale-vector keys |
| `crates/jobsentinel-local-ai/src/evaluation.rs` | Evidence labels, hard negatives, feedback, blockers, and future training data contracts |
| `crates/jobsentinel-local-ai/src/hybrid.rs` | Deterministic hybrid ranking, blocker caps, and retrieval provenance |
| `crates/jobsentinel-local-ai/src/eval_fixtures/seed_v1.json` | Seed labels, hard negatives, and preference pairs for regression tests |
| `crates/jobsentinel-local-ai/src/embeddings.rs` | Embedding generation and vector similarity |
| `crates/jobsentinel-local-ai/src/matcher.rs` | Semantic skill matching logic |
| `crates/jobsentinel-storage/src/resume/matcher/hybrid_score.rs` | Resume/job scoring bridge for hybrid matching and legacy fallback |
| `crates/jobsentinel-local-ai/src/tests.rs` | Feature-gated tests |
| `src-tauri/src/ipc/ml.rs` | Feature-gated Tauri commands |
| `crates/jobsentinel-local-ai/models.lock.toml` | Model supply-chain lockfile |

## Model Direction

The production direction is:

1. Qwen3 embedding retrieval through a JobSentinel-owned model layer.
2. Hybrid scoring with dense retrieval, exact skill taxonomy hits, and BM25.
3. Qwen3 reranking only for bounded top-K candidates.
4. Evidence labels, blockers, and plain explanations instead of unsupported
   fit percentages.
5. Deterministic fallback when local ML is unavailable.

The current Qwen3 embedding backend is `qwen3-candle`, and the bounded reranker
backend is `qwen3-reranker-candle`. Both use JobSentinel's verified cache.
FastEmbed may still be considered as a backend implementation later, but
JobSentinel owns model identity, revisions, cache integrity, runtime
compatibility, vector provenance, thresholds, and scoring semantics. Do not let
a convenience downloader decide production model bytes.

## Evaluation And Improvement

The highest-ROI quality path is supervised ranking work, not reinforcement
learning. Capture and evaluate in this order:

1. Domain eval set with labels: no evidence, keyword-only, related incomplete,
   strong direct evidence, and exceptional ownership.
2. Hard negatives mined from plausible false positives.
3. Qwen3 reranker fine-tuning when labels are sufficient.
4. Embedding fine-tuning only if dense retrieval fails to bring correct
   candidates into top-K.
5. A lightweight learning-to-rank layer over explainable features.
6. Contextual bandit personalization only after real feedback volume exists.

The checked-in seed eval fixture is intentionally small. It proves schema,
coverage, hard-negative shape, role-family expansion, skill-graph confusables,
fairness counterfactuals, self-preference checks, adversarial postings, and
generated-advice separation before scoring changes. Production-quality
evaluation still needs larger reviewed sets for:

- job requirement to resume evidence
- resume profile to job match
- skill phrase to resume evidence
- job title to resume title and seniority
- gap analysis

The current repo-native eval pack is
`crates/jobsentinel-local-ai/src/eval_fixtures/seed_v1.json`, backed by typed contracts
and unit tests. If JobSentinel adds a standalone CLI later, the command surface
should map to retrieval, reranker, fairness, self-preference, and explanation
evals without changing the underlying fixture schema.

The pinned production-path baseline runs all three frozen
`job_requirement_to_resume_evidence` hard negatives through the verified Qwen3
embedding and reranker pair. The reviewed positives produced dense scores of
`0.4218491`, `0.67955077`, and `0.63454574`, then ranked first over their paired
hard negatives, which scored `0.35205674`, `0.4145699`, and `0.48090482`.
The matcher uses a distinct manifest-owned `0.30` retrieval floor so all six
frozen candidates reach the bounded reranker. The test allows at most `0.01`
dense-score drift and requires every candidate to retain at least `0.04`
retrieval margin. The existing `0.48`, `0.65`, and `0.82` weak, medium, and
strong score bands remain unchanged, but the current matcher does not classify
results with those bands.

The paired positive reranker scores were `5.246351`, `5.893036`, and
`6.3774714`; the hard negatives scored `-7.0783405`, `-3.9874773`, and
`1.2468202`. A separate manifest-owned `3.0` acceptance floor prevents a
retrieved candidate from becoming a match solely because it ranked first. The
test allows at most `0.1` reranker-score drift, requires at least `1.0` margin
on each side of the floor, proves each pair selects the positive through
production matching, and proves each hard negative alone leaves the requirement
unmatched. The opt-in ignored test requires an explicit
`JOBSENTINEL_QWEN3_TEST_CACHE`; it does not make model download part of the
default test lane.

This is a seed baseline, not broad calibration evidence. It covers three
synthetic requirement-level pairs on one macOS arm64 host. Larger reviewed
datasets, other query kinds, modest-hardware budgets, and Windows 11, macOS 26,
and Linux release-matrix runs remain required before Gate 4 can close. The
current `0.30` retrieval and `3.0` reranker acceptance values apply only to the
resume-requirement query kind. Weak, medium, and strong score bands, job-search
thresholds, and the MiniLM threshold are not approved user-facing
classifications.

The research-backed evaluation contract is summarized in
[Semantic resume-job matching](../research/semantic-resume-job-matching.md).

Feedback records must be training-friendly and privacy-preserving. Store hashes,
local ids, ranks, scores, evidence classes, and user actions. Do not store raw
resume text, notes, URLs with tokens, browser state, or provider payloads in
diagnostic or training logs.

Preferred improvement path:

1. Improve labels and eval coverage.
2. Mine hard negatives from false positives.
3. Fine-tune the Qwen3 reranker first.
4. Fine-tune Qwen3 embeddings only if retrieval recall stays poor.
5. Add preference learning or a lightweight learning-to-rank layer.
6. Consider bandit-style personalization much later, after enough user
   feedback exists.

## Build and test

Build without local ML:

```bash
cargo build -p jobsentinel --release
```

Build with local ML:

```bash
cargo build -p jobsentinel --release --features embedded-ml
```

Run feature-gated tests:

```bash
cargo test -p jobsentinel --features embedded-ml
```

Run ignored tests that require model files. Downloading tests set the same
anonymous app-scoped download policy the desktop shell sets at startup; that
state is process-global, so run them single-threaded:

```bash
cargo test -p jobsentinel --features embedded-ml -- --ignored --test-threads=1
```

## Commands

This diagnostics command is always registered:

| Command | Purpose |
| ------- | ------- |
| `get_semantic_matching_diagnostics` | Reports local matching mode, Qwen3 model-lock metadata when available, local cache readiness, scoring signals, and privacy-safe quality checks |

These commands are registered only when the app is built with `embedded-ml`:

| Command | Purpose |
| ------- | ------- |
| `download_ml_model` | Native-reviews and starts the governed Qwen3 download with privacy-safe progress |
| `cancel_ml_model_download` | Cancels the active governed model download |
| `remove_ml_models` | Native-reviews and removes all cached data for the governed Qwen3 model identities, including stale revisions and incomplete app-owned staging and transfer data |
| `get_ml_status` | Reports model id, revision, backend, model-lock hash, and whether files are available locally |
| `semantic_match_skills` | Compares user skills with job requirements |
| `match_resume_semantic` | Compares stored resume skills with stored job skills |

The Settings surface exposes the three lifecycle commands only when diagnostics
confirm that the app was built with `embedded-ml`.

## Current matching behavior

The local matcher:

1. Validates visible skill and requirement strings before runtime dispatch.
2. Uses embeddings and bounded reranking when verified local models are
   available.
3. Otherwise matches only case- and whitespace-normalized exact strings,
   without reusing normalized duplicate requirements.
4. Returns matched skills, unmatched requirements with bounded diagnostics,
   unused skills, and an advisory overall result.

Every result names its producing runtime as `qwen3_reranked`, `minilm`, or
`deterministic_exact`. Each skill match keeps `similarity` as its dense cosine
or exact score. Qwen3 matches also retain the selected raw reranker score and
rank as separate fields. Raw reranker scores are query-kind-specific logit
differences, not probabilities, and must not replace or be compared as dense
scores. Unknown or duplicate candidate IDs, missing candidates, non-finite
scores, invalid ranks, invalid dense scores, and inconsistent reranker ordering
fail closed. Embedding and reranker batches require exact input/output
cardinality, and every reranker row must contain exactly one finite scalar.

Every unmatched requirement has exactly one ordered reason:
`no_exact_evidence`, `below_retrieval_threshold`, or
`below_reranker_acceptance`. Matched requirements have no unmatched diagnostic.
The diagnostic contains only the already-returned requirement text and the
closed reason value. It does not expose candidate text, model paths, provider
data, prompts, or raw scores, and it does not claim the user is unqualified.

The current similarity threshold is implementation detail. User-facing copy
should describe outcomes as estimates based on visible evidence, not objective
truth.

## Troubleshooting

| Problem | Safe response |
| ------- | ------------- |
| Model download fails | Keep deterministic matching available and let the user retry later. |
| Model files are absent | Show exact-only local matching and an explicit pinned download action. |
| Model cache is partial, unreadable, unsafe, wrong-size, or checksum-invalid | Report a misconfiguration without paths or file contents, name the active verified fallback, and require a pinned re-download before advanced matching. |
| Runtime dimension differs from vector index | Refuse to use the stale index and rebuild vectors after user-visible setup. |
| Instruction profile changes | Mark existing vectors stale and rebuild before using semantic scores. |
| Reranker is unavailable | Use dense retrieval, exact skill matching, BM25, and mark output as not reranked. |
| Metal acceleration is unavailable | Fall back to CPU inference. |
| Matching output looks wrong | Let the user edit skills and visible assumptions before using the result. |

Do not log raw resume text, private notes, salary floors, local file paths, or
application history while debugging local ML.

## References

- [Candle documentation](https://huggingface.co/docs/candle)
- [Qwen3 Embedding model card](https://huggingface.co/Qwen/Qwen3-Embedding-0.6B)
- [Qwen3 Reranker model card](https://huggingface.co/Qwen/Qwen3-Reranker-0.6B)
- [all-MiniLM-L6-v2 model card](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2)
- [Sentence Transformers](https://www.sbert.net/)
- [BERT paper](https://arxiv.org/abs/1810.04805)
- [Semantic resume-job matching research](../research/semantic-resume-job-matching.md)
- [Hugging Face Candle](https://github.com/huggingface/candle)
- [Hugging Face Hub client](https://github.com/huggingface/hf-hub)
