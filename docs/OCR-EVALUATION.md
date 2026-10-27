# Offline OCR corpus evaluation (Issue #10)

This is an **offline scoring tool**, not a capture tool or a source of measured OCR results. Run the PicPaste Vision helper against your own images on a real Mac; manually transcribe reference text and record each prediction in UTF-8 CSV.

CSV header: `id,language,expected,actual`. `id` is unique, `language` is `zh` or `mixed`, `expected` is a nonempty ground-truth transcription, and `actual` may be empty when nothing was recognized. CSV quoting is required for line breaks and commas in transcription. Do not embed personal or confidential data in public CI artifacts.

```sh
python3 scripts/ocr_eval.py path/to/real-ocr-samples.csv --output ocr-results.json
```

The CLI exits **0 only if every gate passes**, **1 if sample thresholds fail**, **2 for an invalid or unreadable dataset**. JSON reports the real sample count, nonempty-result hit count/rate, and **reference-character-weighted Unicode Levenshtein CER** separately for `zh` and `mixed`. Gates: at least 400 labeled samples, each language category present, Chinese CER <=2%, mixed CER <=5%, and nonempty-result hit rate >=95%. All predictions are evaluated; empty predictions incur deletions. Do not discard failed cases.

**Important metric distinction:** This simple hit-rate measure is the fraction of samples for which OCR returned nonempty text; it does **not** establish word/box selection accuracy or the target UI click-hit rate. Those need independently labeled bounding boxes and real Mac interaction testing. CER uses Unicode code points and does not normalize punctuation, whitespace, or Unicode composition: fix the transcription policy *before* collection and record it with the dataset. Long strings may take significant computation; store the original samples and SHA, macOS version and helper version with any published results.

Unit tests run with the existing macOS Python test step. Fixture-based CI is **not** evidence of 400-image acceptance, actual hardware performance, or macOS TCC/Spaces reliability.
