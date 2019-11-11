# Dex

Dex is a parser for Android's [Dex](https://source.android.com/devices/tech/dalvik/dex-format) format written completely in Rust.

Most of the functionality to access the data structures in the file is implemented but has not been thoroughly tested yet. I plan to write more unit tests, integration tests soon.

## Usage
Add to your `Cargo.toml`:
```
dex = "0.2.1"
```

## Documentation
The primary source of documentation for dex format is [Android website](https://source.android.com/devices/tech/dalvik/dex-format). Most of the public `struct`s, and `method`s in this crate have the same names. There are a few examples [here](https://github.com/letmutx/dex-parser/tree/master/examples/) to get you started.

## Development Notes
* The library makes use of [`mmap`](https://en.wikipedia.org/wiki/Mmap) to access the file contents.
* [scroll](https://crates.io/crates/scroll) is used to parse binary data.
* The included `classes.dex` in the resources folder is from the open-source application [ADW launcher](https://f-droid.org/en/packages/org.adw.launcher/). You can find the source code [here](https://f-droid.org/repo/org.adw.launcher_34_src.tar.gz)

## Contributing
All contributions are welcome! Feel free to raise issues/PRs on Github if you find a bug, have a question or think something can be improved!

There's a TODO list [here](https://github.com/letmutx/dex-parser/tree/master/TODO)
