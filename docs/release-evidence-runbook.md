# PicPaste macOS release-evidence runbook

> **Status: tooling only.** Passing a synthetic unit test or macOS Actions
> runner is NOT proof that physical desktop acceptance has passed. This gate
> helps block premature 1.0 releases and requires separate human review.

## Preconditions and layout

Use the SAME full Git commit for the build and real macOS 14/26 device tests.
Complete the ten true GUI/TCC/IME/Spaces/Retina scenarios in
[native-macos-acceptance.md](native-macos-acceptance.md).
Record original measurements and source hashes. The /evidence/ directory
is ignored by Git because screenshot, OCR and clipboard contents are sensitive.

~~~text
evidence/
  macos14/
    native-acceptance.json
    latency_samples.csv
    evidence-index.md
  macos26/
    native-acceptance.json
    latency_samples.csv
    evidence-index.md
  ocr/
    real-ocr-samples.csv
  scroll-scenarios.csv
  scroll-samples/
    scenario-01.md
  release-gate.json
~~~

For EACH macOS machine, evidence-index.md must have exactly named metadata
fields with a full lowercase 40-digit source SHA, a nonempty real tester and
an ISO8601 timestamp WITH timezone. Replace the example values below:

~~~text
source_sha: 0123456789abcdef0123456789abcdef01234567
tester: macos-qa-owner
observed_at: 2026-10-29T09:00:00+08:00
~~~

The gate checks source_sha against Git HEAD. A missing, duplicated or invalid
metadata field blocks this machine's report.

## Required raw measurements

- latency_samples.csv: header scenario,latency_ms; cold_launch >=30,
  hotkey_to_selection >=100, five_pin_frame >=100. Raw P95 values and counts
  must match the native-acceptance.json figures; the existing acceptance
  verifier checks CPU/RSS and all ten GUI cases.
- ocr/real-ocr-samples.csv: header id,language,expected,actual; >=200 zh and
  >=200 mixed genuine images, >=400 overall; Chinese CER <=2%, mixed CER <=5%,
  nonempty recognition >=95%. Do not discard failed samples.
- scroll-scenarios.csv: exact header scenario_id,category,axis,passed,evidence;
  unique IDs, passed=true|false, evidence as a relative EXISTING path inside
  the same evidence root. Minimum 50 cases distributed 20 static_web,
  10 dynamic_web, 10 document, 5 horizontal, 5 boundary. The horizontal
  category requires horizontal axis; >=95% of all scenarios must succeed.
  Capture 30k/60k/200MP stress and failure recovery evidence.

## Execute on the tested release candidate

~~~bash
python3 scripts/native_acceptance.py verify --report evidence/macos14/native-acceptance.json
python3 scripts/native_acceptance.py verify --report evidence/macos26/native-acceptance.json
python3 scripts/ocr_eval.py evidence/ocr/real-ocr-samples.csv --output evidence/ocr/ocr-results.json
python3 scripts/bench_report.py evidence/macos14/latency_samples.csv --output evidence/macos14/latency_summary.json
python3 scripts/bench_report.py evidence/macos26/latency_samples.csv --output evidence/macos26/latency_summary.json
python3 scripts/release_evidence_gate.py \
  --mac14 evidence/macos14/native-acceptance.json \
  --mac26 evidence/macos26/native-acceptance.json \
  --ocr evidence/ocr/real-ocr-samples.csv \
  --scroll evidence/scroll-scenarios.csv \
  --source-sha "$(git rev-parse HEAD)" \
  --output evidence/release-gate.json
~~~

Exit status 0 means supplied structures, thresholds and source SHA agree.
It DOES NOT authenticate original video/sample provenance. The output always
sets manual_provenance_review_required=true; human inspection of genuine
device recordings, hashes and developer signing is compulsory before release.

## Regression tests and rollback

~~~bash
python3 -m unittest discover -s tests -p 'test_release_evidence_gate.py' -v
python3 -m unittest discover -s tests -p 'test_*.py'
~~~

Tests use synthetic temporary fixtures ONLY to prove the checker rejects
duplicate records, bad metrics, path traversal, malformed metadata and
inadequate OCR balance. Never publish those files as actual QA evidence.
If any real check fails, keep release blocked, fix the software or measurement
setup and repeat the genuine tests. Preserve previous signed stable packages.
