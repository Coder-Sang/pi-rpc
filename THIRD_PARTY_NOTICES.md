# Third-party notices

Original pi-rpc code and documentation are licensed under [Apache-2.0](LICENSE). The wire-compatible implementation follows Pi's published protocol. Protocol examples, fixture shapes and descriptive material informed by Pi retain the following upstream notice. Pi source code and executable distributions are not vendored in this package. See the [provenance review](docs/provenance.md) for the inventory and scope.

## Pi 1.1.0–1.2.0

Reviewed sources: Pi 1.1.0 at [abe508e1b89912adde45528136c3221eb69acdd7](https://github.com/earendil-works/pi/tree/abe508e1b89912adde45528136c3221eb69acdd7) and Pi 1.2.0 at [earendil-works/pi at 06a466935536f8c9bcacb2d5efc0fc021be1bdff](https://github.com/earendil-works/pi/tree/06a466935536f8c9bcacb2d5efc0fc021be1bdff). Both reviewed releases contain identical license bytes. The following text reproduces the latest pinned [upstream LICENSE](https://github.com/earendil-works/pi/blob/06a466935536f8c9bcacb2d5efc0fc021be1bdff/LICENSE).

```text
MIT License

Copyright (c) 2025 Mario Zechner

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Cargo dependencies

Dependencies retain their own licenses; this project's Apache-2.0 declaration does not relicense them. Their sources and binaries are not included in the pi-rpc source crate. Cargo resolves them separately using the shipped manifest and lockfile. The [review record](docs/provenance.md#dependency-metadata) lists the license-expression scope checked for this release candidate.
