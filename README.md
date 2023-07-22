[![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/amachang/path_to_unicode_filename/test.yml?label=test)](https://github.com/amachang/path_to_unicode_filename/actions/workflows/test.yml)
[![Codecov](https://img.shields.io/codecov/c/github/amachang/path_to_unicode_filename)](https://app.codecov.io/gh/amachang/path_to_unicode_filename)
[![docs.rs](https://img.shields.io/docsrs/path_to_unicode_filename)](https://docs.rs/path_to_unicode_filename/latest/path_to_unicode_filename/)
[![Crates.io](https://img.shields.io/crates/l/path_to_unicode_filename)](https://crates.io/crates/path_to_unicode_filename)
[![Crates.io](https://img.shields.io/crates/d/path_to_unicode_filename)](https://crates.io/crates/path_to_unicode_filename)

# path\_to\_unicode\_filename

The library encodes file path separators and common directory names, producing a reversible unicode string that can be used as a filename. It's useful in the case when you want to extract data or features from any file and store them in a specific directory.

It replaces path chars as below:

- chars `\/:*?"<>|` be replaced to full width alternative chars of unicode.
- U+0000 be replaced to `〇`.
- a common directory, like home, documents, pictures, etc are replaced to a OS icon (🍎, 🐧, etc) and a directory icon (🏠, 📄, 🎨, etc).
- chars replacements for others be replaced to twice-sequential chars itself

## Examples

```rust
use path_to_unicode_filename::*;

// make a filename
assert_eq!(to_filename("/tmp/file.txt"), Ok("／tmp／file.txt".into()));
assert_eq!(to_filename("C:\\Users\\alice\\file.txt"), Ok("💠🏠alice＼file.txt".into()));
assert_eq!(to_filename("/Users/alice/Documents/file.txt"), Ok("🍎📄alice／file.txt".into()));

// restore the filename to the original path
assert_eq!(to_path("／var／log／file.txt"), Ok("/var/log/file.txt".into()));
assert_eq!(to_path("🐧🥞sdcard001／file.txt"), Ok("/media/sdcard001/file.txt".into()));
assert_eq!(to_path("🍎🎨bob／file.png"), Ok("/Users/bob/Pictures/file.png".into()));
```

License: MIT OR Apache-2.0
