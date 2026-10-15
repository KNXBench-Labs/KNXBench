"""Git-backed validation of story evidence, isolated from the user's repository."""

from __future__ import annotations

import contextlib
import io
import json
import os
import subprocess

from helpers import TempDirTestCase, load_fixture
from storytool.__main__ import main


class RepositoryReferenceTests(TempDirTestCase):
    def setUp(self) -> None:
        super().setUp()
        self.repo = self.tmp / "repo"
        self.repo.mkdir()
        self.git("init", "-q")
        self.git("config", "user.name", "KNXBench")
        self.git("config", "user.email", "github@knxbench.com")
        (self.repo / "evidence.md").write_text("Synthetic evidence only.\n")
        (self.repo / "folder").mkdir()
        (self.repo / "folder" / "child.md").write_text("Synthetic child.\n")
        self.git("add", ".")
        self.git("commit", "-qm", "Synthetic baseline")
        self.cutoff = self.git("rev-parse", "HEAD")
        self.time = self.git("show", "-s", "--format=%cI", "HEAD")
        self.content = load_fixture()
        self.content["edition"]["cutoff"].update(
            git_commit=self.cutoff, git_commit_time=self.time)
        for event in self.content["events"]:
            event["evidence"] = [
                {"kind": "commit", "label": "Synthetic commit", "ref": self.cutoff,
                 "source": self.content["edition"]["sources"][0]["id"]},
                {"kind": "doc", "label": "Synthetic document", "ref": "evidence.md",
                 "source": self.content["edition"]["sources"][0]["id"]},
            ]

    def git(self, *args: str) -> str:
        env = dict(os.environ, GIT_AUTHOR_NAME="KNXBench", GIT_AUTHOR_EMAIL="github@knxbench.com",
                   GIT_COMMITTER_NAME="KNXBench", GIT_COMMITTER_EMAIL="github@knxbench.com")
        return subprocess.run(["git", *args], cwd=self.repo, env=env, check=True,
                              capture_output=True, text=True).stdout.strip()

    def validate_repository(self) -> tuple[int, str]:
        path = self.tmp / "content.json"
        path.write_text(json.dumps(self.content))
        output = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            try:
                code = main(["validate", "--content", str(path), "--repository", str(self.repo)])
            except SystemExit as error:
                code = error.code
        return code, output.getvalue()

    def test_valid_commit_and_file_are_checked_at_cutoff_not_working_tree(self) -> None:
        (self.repo / "evidence.md").unlink()
        code, output = self.validate_repository()
        self.assertEqual(code, 0, output)
        self.assertIn("repository references checked", output)

    def test_missing_document_is_refused_even_if_untracked_file_exists(self) -> None:
        (self.repo / "untracked.md").write_text("Not in the cutoff.\n")
        self.content["events"][0]["evidence"][1]["ref"] = "untracked.md"
        code, output = self.validate_repository()
        self.assertEqual(code, 1, output)
        self.assertIn("untracked.md", output)
        self.assertIn("not a file at cutoff", output)

    def test_future_commit_is_not_evidence_for_an_earlier_cutoff(self) -> None:
        (self.repo / "future.md").write_text("Later.\n")
        self.git("add", ".")
        self.git("commit", "-qm", "Synthetic later commit")
        self.content["events"][0]["evidence"][0]["ref"] = self.git("rev-parse", "HEAD")
        code, output = self.validate_repository()
        self.assertEqual(code, 1, output)
        self.assertIn("not an ancestor of cutoff", output)

    def test_missing_commit_is_refused(self) -> None:
        self.content["events"][0]["evidence"][0]["ref"] = "0" * 40
        code, output = self.validate_repository()
        self.assertEqual(code, 1, output)
        self.assertIn("missing or ambiguous commit", output)

    def test_cutoff_time_must_match_git_including_timezone(self) -> None:
        self.content["edition"]["cutoff"]["git_commit_time"] = "2000-01-01T00:00:00+00:00"
        code, output = self.validate_repository()
        self.assertEqual(code, 1, output)
        self.assertIn("cutoff commit time", output)

    def test_directory_is_not_a_file_reference(self) -> None:
        self.content["events"][0]["evidence"][1]["ref"] = "folder"
        code, output = self.validate_repository()
        self.assertEqual(code, 1, output)
        self.assertIn("not a file at cutoff", output)
