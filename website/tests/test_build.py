"""Contract tests for the offline marketing companion (stdlib only)."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from html.parser import HTMLParser

WEBSITE = Path(__file__).resolve().parents[1]


class Document(HTMLParser):
    def __init__(self, text):
        super().__init__()
        self.elements = []
        self.feed(text)

    def handle_starttag(self, tag, attrs):
        self.elements.append((tag, dict(attrs)))


class BuildTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="website-test-", dir=os.environ.get("TMPDIR"))
        self.addCleanup(self.temp.cleanup)
        self.out = Path(self.temp.name) / "site"

    def build(self, *args):
        return subprocess.run([sys.executable, str(WEBSITE / "build.py"), "--output", str(self.out), *args],
                              capture_output=True, text=True)

    def require_build(self):
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_build_has_complete_german_and_english_pages(self):
        self.require_build()
        for lang in ("de", "en"):
            page = (self.out / lang / "index.html").read_text()
            doc = Document(page)
            self.assertIn(("html", {"lang": lang}), doc.elements)
            for section in ("features", "watch", "personality", "evolution", "status", "start"):
                self.assertTrue(any(attrs.get("id") == section for _, attrs in doc.elements), section)
            self.assertEqual(sum(tag == "h1" for tag, _ in doc.elements), 1)
            self.assertNotIn("${", page)

    def test_root_defaults_to_english_with_explicit_language_links(self):
        self.require_build()
        page = (self.out / "index.html").read_text()
        doc = Document(page)
        self.assertIn(("html", {"lang": "en"}), doc.elements)
        self.assertIn("Your KNX project.", page)
        self.assertIn("Your workbench.", page)
        self.assertIn('href="https://knxbench.com/en/"', page)
        self.assertIn('href="en/contact/">Legal notice</a>', page)
        self.assertIn('src="assets/headlines.js"', page)
        self.assertTrue(any(tag == "a" and attrs.get("lang") == "en" and
                            attrs.get("aria-current") == "page"
                            for tag, attrs in doc.elements))
        for lang in ("de", "en"):
            self.assertIn(f'href="{lang}/"', page)
            explicit = Document((self.out / lang / "index.html").read_text())
            self.assertIn(("html", {"lang": lang}), explicit.elements)

    def test_build_is_byte_deterministic_and_can_replace_its_own_output(self):
        self.require_build()
        before = {str(p.relative_to(self.out)): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in self.out.rglob("*") if p.is_file()}
        self.require_build()
        after = {str(p.relative_to(self.out)): hashlib.sha256(p.read_bytes()).hexdigest()
                 for p in self.out.rglob("*") if p.is_file()}
        self.assertEqual(before, after)

    def test_unknown_output_files_are_not_deleted(self):
        self.out.mkdir()
        sentinel = self.out / "user-project.knxdb"
        sentinel.write_bytes(b"not owned by the website builder")
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(sentinel.read_bytes(), b"not owned by the website builder")

    def test_even_owned_output_refuses_unlisted_files(self):
        self.require_build()
        sentinel = self.out / "unrelated.txt"
        sentinel.write_text("keep me")
        self.assertNotEqual(self.build().returncode, 0)
        self.assertEqual(sentinel.read_text(), "keep me")

    def test_owned_output_with_edited_file_is_preserved(self):
        self.require_build()
        page = self.out / "de/index.html"
        page.write_text("user-edited page, preserve this")
        self.assertNotEqual(self.build().returncode, 0)
        self.assertEqual(page.read_text(), "user-edited page, preserve this")

    def test_output_symlink_is_refused(self):
        external = Path(self.temp.name) / "other"
        external.mkdir()
        self.out.symlink_to(external, target_is_directory=True)
        self.assertNotEqual(self.build().returncode, 0)
        self.assertTrue(self.out.is_symlink())

    def test_release_build_is_public_and_bound_to_the_story_approval(self):
        result = self.build("--release")
        self.assertEqual(result.returncode, 0, result.stderr)
        manifest = json.loads((self.out / "build-manifest.json").read_text())
        self.assertEqual(manifest["mode"], "release")
        self.assertEqual((self.out / "CNAME").read_text(), "knxbench.com\n")
        self.assertEqual((self.out / "robots.txt").read_text(), "User-agent: *\nAllow: /\n")
        for page in self.out.rglob("*.html"):
            html = page.read_text()
            self.assertNotIn("noindex", html, page)
            self.assertNotIn('class="preview-note"', html, page)
            self.assertNotIn('class="launch-note"', html, page)
            self.assertNotIn('class="preview-banner"', html, page)
            self.assertNotIn("KNXBench-Contributions", html, page)
        self.assertIn("<span>Published edition</span>", (self.out / "story/index.html").read_text())
        for lang, title in (("de", "Datenschutz"), ("en", "Privacy")):
            privacy = (self.out / lang / "privacy/index.html").read_text()
            self.assertIn(f"<h1>{title}</h1>", privacy)
            self.assertIn("GitHub Pages", privacy)
            self.assertIn('href="https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement"', privacy)

    def test_release_refuses_a_mismatched_story_approval_before_writing(self):
        approval = Path(self.temp.name) / "approval.json"
        approval.write_text(json.dumps({"schema": "knxbench-evolution-approval/1", "decision": "publish-exact-version",
                                        "candidate_id": "2026-10-08.4", "story_sha256": "0" * 64,
                                        "approved_by": "x", "approved_at": "x"}))
        result = self.build("--release", "--story-approval", str(approval))
        self.assertEqual(result.returncode, 1)
        self.assertIn("approval", result.stderr)
        self.assertFalse(self.out.exists())

    def test_story_approval_override_only_applies_to_release(self):
        result = self.build("--story-approval", "x.json")
        self.assertEqual(result.returncode, 2)
        self.assertFalse(self.out.exists())

    def test_preview_stays_unindexed_and_marked(self):
        self.require_build()
        self.assertEqual((self.out / "robots.txt").read_text(), "User-agent: *\nDisallow: /\n")
        self.assertFalse((self.out / "CNAME").exists())
        for lang in ("de", "en"):
            html = (self.out / lang / "index.html").read_text()
            self.assertIn("noindex", html)
            self.assertIn('class="preview-note"', html)

    def test_page_assets_are_local_exist_and_videos_do_not_autoplay(self):
        self.require_build()
        for lang in ("de", "en"):
            doc = Document((self.out / lang / "index.html").read_text())
            videos = []
            for tag, attrs in doc.elements:
                if tag in ("script", "img", "source", "track", "video"):
                    for key in ("src", "poster"):
                        if key in attrs:
                            self.assertNotIn("://", attrs[key])
                            self.assertTrue((self.out / lang / attrs[key]).resolve().is_file(), attrs[key])
                if tag == "video":
                    videos.append(attrs)
                    self.assertNotIn("autoplay", attrs)
                    self.assertNotIn("loop", attrs)
                    self.assertIn("controls", attrs)
                    self.assertEqual(attrs["preload"], "none")
                self.assertNotEqual(tag, "iframe")
            self.assertGreaterEqual(len(videos), 2)

    def test_manifest_contains_only_explicit_public_artifacts(self):
        self.require_build()
        manifest = json.loads((self.out / "build-manifest.json").read_text())
        self.assertEqual(manifest["mode"], "private-preview")
        self.assertEqual(manifest["schema"], "knxbench-website-build/1")
        actual = sorted(str(p.relative_to(self.out)) for p in self.out.rglob("*") if p.is_file()
                        and p.name != "build-manifest.json")
        self.assertEqual(sorted(manifest["files"]), actual)
        for name, digest in manifest["files"].items():
            self.assertEqual(hashlib.sha256((self.out / name).read_bytes()).hexdigest(), digest)
            self.assertNotIn(".ai/", name)
            self.assertNotIn("node_modules", name)
            self.assertNotIn("candidates/", name)
        self.assertTrue((self.out / "story/index.html").is_file())

    def test_contact_pages_link_owner_provided_email(self):
        self.require_build()
        for lang in ("de", "en"):
            page = (self.out / lang / "contact/index.html").read_text()
            doc = Document(page)
            self.assertTrue(any(tag == "a" and attrs.get("href") ==
                                "mailto:contact@knxbench.com" for tag, attrs in doc.elements))
            self.assertIn(">contact@knxbench.com</a>", page)
        for lang in ("de", "en"):
            privacy = (self.out / lang / "privacy/index.html").read_text()
            self.assertNotIn("mailto:contact@knxbench.com", privacy)

    def test_imprint_contains_exact_owner_details_and_footer_link(self):
        self.require_build()
        for lang, title in (("de", "Impressum"), ("en", "Legal notice")):
            page = (self.out / lang / "contact/index.html").read_text()
            self.assertIn(f"<h1>{title}</h1>", page)
            self.assertIn("<address", page)
            for supplied in ("Andre Becker", "Schusterstrasse 3", "48268 Greven"):
                self.assertIn(supplied, page)
            self.assertNotIn('class="launch-note"', page)
            landing = (self.out / lang / "index.html").read_text()
            self.assertIn(f'contact/">{title}</a>', landing)
        root_page = (self.out / "index.html").read_text()
        self.assertIn('en/contact/">Legal notice</a>', root_page)

    def test_languages_have_identical_translation_keys(self):
        de = json.loads((WEBSITE / "content/de.json").read_text())
        en = json.loads((WEBSITE / "content/en.json").read_text())
        self.assertEqual(set(de), set(en))
        self.assertTrue(all(isinstance(v, str) and v for v in de.values()))
        self.assertTrue(all(isinstance(v, str) and v for v in en.values()))

    def test_duplicate_json_fields_are_refused(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location("website_builder", WEBSITE / "build.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        source = Path(self.temp.name) / "ambiguous.json"
        source.write_text('{"edition": "first", "edition": "second"}')
        with self.assertRaises(module.BuildError):
            module.read_json(source)

    def test_preview_server_refuses_network_bind_before_listening(self):
        result = subprocess.run([sys.executable, str(WEBSITE / "serve.py"), "--host", "0.0.0.0"],
                                capture_output=True, text=True, timeout=5)
        self.assertEqual(result.returncode, 3)
        self.assertIn("loopback", result.stderr)

    def test_output_cannot_replace_website_sources(self):
        result = subprocess.run([sys.executable, str(WEBSITE / "build.py"), "--output", str(WEBSITE)],
                                capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((WEBSITE / "page.html").is_file())

    def test_runtime_has_no_tracking_storage_or_remote_calls(self):
        self.require_build()
        js = "\n".join((self.out / "assets" / name).read_text()
                       for name in ("site.js", "headlines.js"))
        for forbidden in ("fetch(", "XMLHttpRequest", "localStorage", "sessionStorage", "document.cookie", "sendBeacon"):
            self.assertNotIn(forbidden, js)
        self.assertTrue((self.out / ".nojekyll").is_file())


if __name__ == "__main__":
    unittest.main()
