"""Keep the Linux server port and documented host-network recipes consistent."""

from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
GUIDES = (
    "README.md",
    "apps/knx-server/DOCKERHUB.md",
    "docs/manual/getting-started/04-installation.md",
    "docs/manual/user-guide/11-web-and-docker.md",
)


def server_commands(text):
    """Inspect complete continued commands, excluding one-shot image utilities."""
    joined = text.replace("\\\n", " ")
    return re.findall(r"^.*docker run -d\b[^\n]*", joined, re.MULTILINE)


class ServerDeploymentDefaultsTests(unittest.TestCase):
    def test_image_listens_on_and_exposes_8484(self):
        dockerfile = (ROOT / "apps/knx-server/Dockerfile").read_text()
        self.assertRegex(dockerfile, r"(?m)^ENV KNX_PORT=8484$")
        self.assertRegex(dockerfile, r"(?m)^EXPOSE 8484$")

    def test_product_database_lives_on_the_data_volume(self):
        # GitHub #21: a product database outside /data vanished with every
        # container rebuild while the saved projects survived on the volume.
        dockerfile = (ROOT / "apps/knx-server/Dockerfile").read_text()
        self.assertRegex(dockerfile, r"(?m)^VOLUME /data$")
        match = re.search(r"(?m)^ENV XDG_DATA_HOME=(\S+)$", dockerfile)
        self.assertIsNotNone(match, "XDG_DATA_HOME must be set in the image")
        self.assertTrue(match.group(1).startswith("/data/"), match.group(1))

    def test_all_documented_server_starts_use_host_networking(self):
        for guide in GUIDES:
            commands = server_commands((ROOT / guide).read_text())
            self.assertTrue(commands, guide)
            for command in commands:
                with self.subTest(guide=guide, command=command):
                    self.assertIn("--network host", command)
                    self.assertNotRegex(command, r"(?:^|\s)(?:-p|--publish)(?:\s|=)")

    def test_downloaded_image_recipes_keep_old_releases_on_8484(self):
        for guide in GUIDES:
            for command in server_commands((ROOT / guide).read_text()):
                if "knxbench/knxbench-server:" in command:
                    with self.subTest(guide=guide, command=command):
                        self.assertIn("-e KNX_PORT=8484", command)

    def test_smoke_test_uses_host_network_with_an_isolated_port(self):
        script = (ROOT / "apps/knx-server/scripts/smoke-test.sh").read_text()
        commands = server_commands(script)
        self.assertEqual(len(commands), 1)
        self.assertIn("--network host", commands[0])
        self.assertIn('-e KNX_PORT="$port"', commands[0])
        self.assertNotRegex(commands[0], r"(?:^|\s)(?:-p|--publish)(?:\s|=)")
        self.assertIn('port="${KNXBENCH_SMOKE_PORT:-18080}"', script)

    def test_current_reader_guides_do_not_advertise_8080(self):
        guides = (*GUIDES,
                  "docs/manual/development/02-building-from-source.md",
                  "docs/manual/reference/03-troubleshooting.md")
        for guide in guides:
            with self.subTest(guide=guide):
                self.assertNotRegex((ROOT / guide).read_text(), r"\b8080\b")


if __name__ == "__main__":
    unittest.main()
