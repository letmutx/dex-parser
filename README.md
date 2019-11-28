# Dex

[![Build Status](https://api.travis-ci.org/letmutx/dex-parser.svg?branch=master)](https://travis-ci.org/letmutx/dex-parser)

Dex is a parser for Android's [Dex](https://source.android.com/devices/tech/dalvik/dex-format) format written completely in Rust.

Most of the functionality to access the data structures in the file is implemented but has not been thoroughly tested yet. I plan to write more unit tests, integration tests soon.

## Usage
Add to your `Cargo.toml`:
```
dex = "0.2.4"
```

## Documentation
The primary source of documentation for dex format is [Android website](https://source.android.com/devices/tech/dalvik/dex-format). Most of the public `struct`s, and `method`s in this crate have the same names. There are a few examples [here](https://github.com/letmutx/dex-parser/tree/master/examples/) to get you started.

## Development Notes
* The library makes use of [`mmap`](https://en.wikipedia.org/wiki/Mmap) to access the file contents.
* [scroll](https://crates.io/crates/scroll) is used to parse binary data.
* The included `classes.dex` in the resources folder is from the open-source application [ADW launcher](https://f-droid.org/en/packages/org.adw.launcher/). You can find the source code [here](https://f-droid.org/repo/org.adw.launcher_34_src.tar.gz)

## Running test cases
Some tests contains Java code and require `javac` and [d8](https://developer.android.com/studio/command-line/d8).

* To get `d8`, you need to install Android SDK and add `Android/Sdk/build-tools/<version>/` directory to PATH variable.
* For `javac`, you need to install Java.
* Also, `ANDROID_LIB_PATH` variable needs to be set in the environment. It should point to the `android.jar` file in the SDK. (ex: `Android/Sdk/platforms/android-<version>/android.jar`). This is needed to prevent warnings when running `d8`.

## Contributing
All contributions are welcome! Feel free to raise issues/PRs on Github if you find a bug, have a question or think something can be improved!

There's a TODO list [here](https://github.com/letmutx/dex-parser/tree/master/TODO)
