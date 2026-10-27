import csv
import tempfile
import unittest
from pathlib import Path

from scripts.ocr_eval import evaluate, character_errors


class OcrEvaluationTests(unittest.TestCase):
    def test_unicode_edit_distance_handles_insert_delete_substitute(self):
        self.assertEqual(character_errors("中文ABC", "中文AXC"), 1)
        self.assertEqual(character_errors("汉字", "汉"), 1)
        self.assertEqual(character_errors("", "甲乙"), 2)

    def test_aggregates_by_language_weighted_characters_and_hits(self):
        with tempfile.TemporaryDirectory() as td:
            csv_path = Path(td) / "samples.csv"
            with csv_path.open("w", encoding="utf-8", newline="") as stream:
                writer = csv.writer(stream)
                writer.writerow(["id", "language", "expected", "actual"])
                writer.writerow(["a", "zh", "中文甲乙", "中文甲乙"])
                writer.writerow(["b", "zh", "测试", "测错"])
                writer.writerow(["c", "mixed", "A文B", "A文B"])
                writer.writerow(["d", "mixed", "Z字", ""])
            summary = evaluate(csv_path)
            self.assertEqual(summary["overall"]["samples"], 4)
            self.assertEqual(summary["overall"]["hits"], 3)
            self.assertEqual(summary["overall"]["hit_rate"], 0.75)
            self.assertEqual(summary["zh"]["characters"], 6)
            self.assertEqual(summary["zh"]["errors"], 1)
            self.assertAlmostEqual(summary["zh"]["cer"], 1 / 6)
            self.assertEqual(summary["mixed"]["characters"], 5)
            self.assertEqual(summary["mixed"]["errors"], 2)
            self.assertEqual(summary["acceptance"]["passed"], False)
            self.assertEqual(summary["acceptance"]["reasons"], ["requires >=400 samples", "Chinese CER >2%", "mixed CER >5%", "hit rate <95%"])

    def test_rejects_empty_truth_duplicate_ids_and_unknown_language(self):
        with tempfile.TemporaryDirectory() as td:
            csv_path = Path(td) / "invalid.csv"
            for content in (
                "id,language,expected,actual\na,zh,,X\n",
                "id,language,expected,actual\na,zh,X,X\na,mixed,Y,Y\n",
                "id,language,expected,actual\na,en,X,X\n",
                "id,language,expected,actual\na,zh,X\n",
            ):
                csv_path.write_text(content, encoding="utf-8")
                with self.assertRaises(ValueError):
                    evaluate(csv_path)

    def test_no_data_does_not_pass_threshold(self):
        with tempfile.TemporaryDirectory() as td:
            csv_path = Path(td) / "samples.csv"
            csv_path.write_text("id,language,expected,actual\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                evaluate(csv_path)


if __name__ == "__main__":
    unittest.main()
