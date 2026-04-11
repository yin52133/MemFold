import re
import unittest
from pathlib import Path


ROOT = Path("/home/ps/project/MemFold")
SPECS = ROOT / "docs" / "specs"
REFS = ROOT / "docs" / "references"

INDEX_DOC = SPECS / "2026-04-11-memfold-architecture-design.zh-CN.md"
DETAIL_DIR = SPECS / "2026-04-11-memfold"

DETAIL_DOCS = {
    "01-system-overview.zh-CN.md": 220,
    "02-loading-and-retrieval.zh-CN.md": 220,
    "03-storage-and-qmd.zh-CN.md": 220,
    "04-dreaming-and-autoresearch.zh-CN.md": 220,
    "05-runtime-and-implementation.zh-CN.md": 180,
}


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


class TestDesignDocs(unittest.TestCase):
    def test_required_files_exist(self):
        self.assertTrue(INDEX_DOC.exists())
        self.assertTrue((REFS / "memfold-open-source-review.zh-CN.md").exists())
        for name in DETAIL_DOCS:
            self.assertTrue((DETAIL_DIR / name).exists(), name)

    def test_index_doc_is_short_navigation_not_full_dump(self):
        text = read(INDEX_DOC)
        self.assertLessEqual(len(text.splitlines()), 120)
        self.assertIn("文档地图", text)
        self.assertNotIn("License 判断", text)
        self.assertNotIn("借鉴映射", text)

    def test_detail_docs_have_line_budget(self):
        for name, max_lines in DETAIL_DOCS.items():
            text = read(DETAIL_DIR / name)
            self.assertLessEqual(
                len(text.splitlines()),
                max_lines,
                f"{name} exceeds line budget {max_lines}",
            )

    def test_spec_docs_do_not_contain_history_debate_sections(self):
        banned_patterns = [
            r"为什么不是 Python",
            r"为什么不是 Go",
            r"License 判断",
            r"借鉴映射",
        ]
        docs = [INDEX_DOC] + [DETAIL_DIR / name for name in DETAIL_DOCS]
        for path in docs:
            text = read(path)
            for pattern in banned_patterns:
                self.assertIsNone(
                    re.search(pattern, text),
                    f"{path.name} contains banned pattern: {pattern}",
                )

    def test_core_docs_have_diagrams(self):
        diagram_required = [
            DETAIL_DIR / "01-system-overview.zh-CN.md",
            DETAIL_DIR / "02-loading-and-retrieval.zh-CN.md",
            DETAIL_DIR / "03-storage-and-qmd.zh-CN.md",
            DETAIL_DIR / "04-dreaming-and-autoresearch.zh-CN.md",
        ]
        for path in diagram_required:
            self.assertIn("```mermaid", read(path), path.name)

    def test_loading_doc_contains_two_routes_and_mode_matrix(self):
        text = read(DETAIL_DIR / "02-loading-and-retrieval.zh-CN.md")
        self.assertIn("模式矩阵", text)
        self.assertIn("默认检索路径", text)
        self.assertIn("显式知识检索路径", text)
        self.assertIn("语义墓碑", text)

    def test_loading_doc_has_boot_budget_and_slots(self):
        text = read(DETAIL_DIR / "02-loading-and-retrieval.zh-CN.md")
        self.assertIn("Boot Bundle", text)
        self.assertIn("slot", text.lower())
        self.assertIn("normal", text)
        self.assertIn("sterile", text)

    def test_loading_doc_blocks_reflection_from_boot(self):
        text = read(DETAIL_DIR / "02-loading-and-retrieval.zh-CN.md")
        self.assertIn("reflection note", text.lower())
        self.assertIn("禁止进入 Boot", text)

    def test_loading_doc_has_source_gating(self):
        text = read(DETAIL_DIR / "02-loading-and-retrieval.zh-CN.md")
        self.assertIn("Source Gating", text)
        self.assertIn("effective", text)
        self.assertIn("knowledge_lookup", text)

    def test_storage_doc_has_truth_source_and_mutation_protocol(self):
        text = read(DETAIL_DIR / "03-storage-and-qmd.zh-CN.md")
        self.assertIn("内容真相源", text)
        self.assertIn("状态真相源", text)
        self.assertIn("跨存储提交与恢复协议", text)
        self.assertIn("QMD 文档模型契约", text)

    def test_storage_doc_defines_observation_source_of_truth(self):
        text = read(DETAIL_DIR / "03-storage-and-qmd.zh-CN.md")
        self.assertIn("Observation 的真相源", text)
        self.assertIn("append-only JSONL", text)

    def test_storage_doc_defines_memory_item_unit(self):
        text = read(DETAIL_DIR / "03-storage-and-qmd.zh-CN.md")
        self.assertIn("Memory Item 的最小单位", text)
        self.assertIn("item_key", text)
        self.assertIn("content_hash", text)

    def test_storage_doc_says_qmd_is_not_source_of_truth(self):
        text = read(DETAIL_DIR / "03-storage-and-qmd.zh-CN.md")
        self.assertIn("QMD 是检索 sidecar，不是状态真相源", text)

    def test_storage_doc_defines_lock_source(self):
        text = read(DETAIL_DIR / "03-storage-and-qmd.zh-CN.md")
        self.assertIn("锁真相源只允许一个", text)
        self.assertIn("SQLite lease", text)

    def test_dreaming_doc_has_ingress_gate_and_state_machine(self):
        text = read(DETAIL_DIR / "04-dreaming-and-autoresearch.zh-CN.md")
        self.assertIn("Observation Ingress Gate", text)
        self.assertIn("Reflection Note", text)
        self.assertIn("Dreaming 状态机", text)
        self.assertIn("hard gate", text)

    def test_dreaming_doc_separates_reflection_note(self):
        text = read(DETAIL_DIR / "04-dreaming-and-autoresearch.zh-CN.md")
        self.assertIn("Reflection Note", text)
        self.assertIn("不直接消费 Reflection Note", text)

    def test_dreaming_doc_has_experiment_isolation(self):
        text = read(DETAIL_DIR / "04-dreaming-and-autoresearch.zh-CN.md")
        self.assertIn("实验环境硬隔离", text)
        self.assertIn("只读内容快照", text)
        self.assertIn("独立 SQLite 状态路径", text)

    def test_overview_doc_defines_boot_as_derived_view(self):
        text = read(DETAIL_DIR / "01-system-overview.zh-CN.md")
        self.assertIn("Boot Layer", text)
        self.assertIn("加载视图", text)
        self.assertIn("不是独立真相源", text)

    def test_overview_doc_has_non_goals(self):
        text = read(DETAIL_DIR / "01-system-overview.zh-CN.md")
        self.assertIn("非目标", text)
        self.assertIn("纯 wiki 产品", text)

    def test_runtime_doc_has_integration_protocol(self):
        text = read(DETAIL_DIR / "05-runtime-and-implementation.zh-CN.md")
        self.assertIn("最小集成协议", text)
        self.assertIn("load(mode, scope, intent)", text)

    def test_runtime_doc_has_v1_scope(self):
        text = read(DETAIL_DIR / "05-runtime-and-implementation.zh-CN.md")
        self.assertIn("V1 必做", text)
        self.assertIn("observation JSONL + SQLite 投影", text)

    def test_reference_doc_holds_license_review(self):
        text = read(REFS / "memfold-open-source-review.zh-CN.md")
        self.assertIn("License", text)
        self.assertIn("AGPL-3.0-or-later", text)
        self.assertIn("Claude-Mem", text)


if __name__ == "__main__":
    unittest.main()
