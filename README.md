[![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/amachang/path_to_unicode_filename/test.yml?label=test)](https://github.com/amachang/path_to_unicode_filename/actions/workflows/test.yml)
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
to_filename("/tmp/file.txt") =>  // => ／tmp／file.txt

to_filename("C:\\Users\\alice\\file.txt") // => 💠🏠alice＼file.txt

to_filename("/Users/alice/Documents/file.txt") // => 🍎📄alice／file.txt
```

License: MIT OR Apache-2.0
