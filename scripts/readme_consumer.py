"""README caller-project generation and feature-aware Rustdoc checks."""
import json
from html.parser import HTMLParser
from pathlib import Path
import re
import tomllib

from validation_checks import require


def toml_dependencies(dependencies):
    def scalar(item):
        # TOML forbids raw DEL; keep non-BMP scalars out of JSON surrogate pairs.
        return json.dumps(item, ensure_ascii=False).replace("\x7f", "\\u007f")

    def value(item):
        if isinstance(item, dict):
            return "{ " + ", ".join(scalar(key) + " = " + value(entry)
                                     for key, entry in item.items()) + " }"
        if isinstance(item, list):
            return "[" + ", ".join(value(entry) for entry in item) + "]"
        require(isinstance(item, (str, bool)), "unsupported README dependency value")
        return scalar(item)

    return "[dependencies]\n" + "".join(
        scalar(name) + " = " + value(specification) + "\n"
        for name, specification in dependencies.items())


def readme_consumer_sources(readme, package_root, version):
    def block(language):
        matches = re.findall(r"^```" + re.escape(language) + r"\s*\n(.*?)^```\s*$",
                             readme, re.MULTILINE | re.DOTALL)
        require(len(matches) == 1, f"README must contain one {language} example")
        return matches[0]

    dependencies = block("toml")
    manifest = tomllib.loads(dependencies)
    require(set(manifest) == {"dependencies"}, "README dependency example must be standalone")
    specification = manifest["dependencies"].get("pi-rpc")
    require(isinstance(specification, (str, dict)), "README must declare the pi-rpc dependency")
    options = dict(specification) if isinstance(specification, dict) else {"version": specification}
    require("workspace" not in options, "README dependency must not require a workspace")
    for source in ("path", "git", "branch", "tag", "rev", "registry"):
        options.pop(source, None)
    options["path"] = Path(package_root).resolve().as_posix()
    options.setdefault("version", "=" + version)
    manifest["dependencies"]["pi-rpc"] = options
    dependencies = toml_dependencies(manifest["dependencies"])

    example = block("rust,no_run")
    require(re.search(r"\basync fn inspect_session\s*\(", example) is not None,
            "README must define inspect_session")
    # Referencing the function outside the copied example catches a cfg that
    # silently removes it. Never run it: its body starts a real Pi process.
    sources = {
        "Cargo.toml": '[package]\nname = "pi-rpc-readme-consumer"\nversion = "0.0.0"\n'
                      'edition = "2024"\nrust-version = "1.88"\npublish = false\n\n' + dependencies,
        "src/main.rs": example + "\nfn main() { let _ = inspect_session; }\n",
    }
    require("features" not in tomllib.loads(sources["Cargo.toml"]),
            "README consumer must not declare local features")
    return sources


def check_readme_rustdoc(doc_root):
    class Page(HTMLParser):
        def __init__(self):
            super().__init__()
            self.ids = set()
            self.examples = []
            self.code = None

        def handle_starttag(self, tag, attrs):
            self.ids.update(value for key, value in attrs if key == "id")
            if tag == "code":
                self.code = []

        def handle_data(self, data):
            if self.code is not None:
                self.code.append(data)

        def handle_endtag(self, tag):
            if tag == "code" and self.code is not None:
                text = "".join(self.code)
                if "async fn inspect_session" in text:
                    self.examples.append(text)
                self.code = None

    page = Page()
    page.feed((Path(doc_root) / "pi_rpc/index.html").read_text(encoding="utf-8"))
    available = {"reexport.Pi15", "reexport.PiProcess"}.issubset(page.ids)
    require(len(page.examples) == int(available),
            "Rustdoc README example must follow the available Pi15/process APIs")
    require(all("#[cfg(" not in example for example in page.examples),
            "Rustdoc README example must not contain caller feature gates")
    return available
