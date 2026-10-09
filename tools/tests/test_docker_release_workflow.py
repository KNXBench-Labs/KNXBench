"""Keep authenticated base-image pulls separate from release publication."""

from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/docker-release.yml"


class DockerReleaseWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.workflow = WORKFLOW.read_text(encoding="utf-8")
        self.build, self.publish = self.workflow.split("\n  publish:", 1)

    def step(self, name):
        prefix = f"      - name: {name}\n"
        self.assertEqual(self.build.count(prefix), 1)
        return self.build.split(prefix, 1)[1].split("\n      - ", 1)[0]

    def test_login_precedes_the_first_base_image_pull(self):
        self.assertLess(
            self.build.index("name: Log in to Docker Hub"),
            self.build.index("name: Build for the smoke test"),
        )

    def test_configured_credentials_can_authenticate_dry_run_pulls(self):
        self.assertIn(
            "HAVE_DOCKERHUB_LOGIN: ${{ vars.DOCKERHUB_USERNAME != '' && secrets.DOCKERHUB_TOKEN != '' }}",
            self.build,
        )
        self.assertIn(
            "if: env.PUBLISH == 'true' || env.HAVE_DOCKERHUB_LOGIN == 'true'",
            self.step("Log in to Docker Hub"),
        )

    def test_only_credential_presence_not_values_is_added_to_job_environment(self):
        job_environment = self.build.split("    env:\n", 1)[1].split("    steps:\n", 1)[0]
        self.assertIn("HAVE_DOCKERHUB_LOGIN:", job_environment)
        self.assertNotIn("DOCKERHUB_TOKEN: ${{ secrets.DOCKERHUB_TOKEN }}", job_environment)

    def test_dry_runs_still_cannot_publish(self):
        self.assertIn("PUBLISH: ${{ github.event_name == 'push' && github.ref_type == 'tag' }}", self.build)
        for name in ("Push by digest", "Record digest", "Upload digest"):
            with self.subTest(step=name):
                self.assertIn("if: env.PUBLISH == 'true'", self.step(name))
        self.assertIn("if: github.event_name == 'push' && github.ref_type == 'tag'", self.publish)


if __name__ == "__main__":
    unittest.main()
