import unittest
from version import components, prepare, validate_tag

MANIFEST = '[package]\nname = "rustedbytes-tl"\nversion = "0.2.0" # retained\n\n[dependencies]\nother = { version = "0.2.0" }\n'


class VersionTests(unittest.TestCase):
    def test_bumps_preserve_dependencies_and_comments(self):
        for bump, expected in (("patch", "0.2.1"), ("minor", "0.3.0"), ("major", "1.0.0")):
            with self.subTest(bump=bump):
                text, version = prepare(MANIFEST, bump, "")
                self.assertEqual(version, expected)
                self.assertEqual(text, MANIFEST.replace('version = "0.2.0" #', f'version = "{expected}" #'))

    def test_explicit_version_overrides_component(self):
        self.assertEqual(prepare(MANIFEST, "patch", "1.2.3")[1], "1.2.3")

    def test_rejects_noncanonical_versions(self):
        for value in ("01.2.3", "v1.2.3", "1.2", "1.2.3-rc.1", "1.2.3+build", "1.2.3\n", "$(id)"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                components(value)

    def test_rejects_reused_or_decreased_versions(self):
        for value in ("0.2.0", "0.1.99"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                prepare(MANIFEST, "patch", value)

    def test_release_tag_must_match_exactly(self):
        self.assertEqual(validate_tag(MANIFEST, "v0.2.0"), "0.2.0")
        for tag in ("0.2.0", "v0.3.0", "v0.2.0-rc.1"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                validate_tag(MANIFEST, tag)


if __name__ == "__main__":
    unittest.main()
