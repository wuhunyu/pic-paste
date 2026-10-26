import tempfile
import unittest
from pathlib import Path
from scripts.bench_report import summarize

class BenchTests(unittest.TestCase):
    def test_p95_interpolation(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "samples.csv"
            path.write_text("scenario,latency_ms\nhotkey,10\nhotkey,20\nhotkey,30\n", encoding="utf-8")
            self.assertEqual(summarize(path)["hotkey"], {
                "samples": 3, "p50_ms": 20.0, "p95_ms": 29.0, "max_ms": 30.0
            })

    def test_rejects_invalid_numbers(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "samples.csv"
            path.write_text("scenario,latency_ms\nstart,nan\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                summarize(path)

if __name__ == "__main__":
    unittest.main()
