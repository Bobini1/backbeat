# Backbeat C SDK

A C Interface for the `backbeat_sdk` API.

Documentation for functions is inside this crate. If you're not using Rust, this is the way you should interface with backbeat.

You can build sensible wrappers ontop of this for your language, as every language has good C FFI support. See the Java SDK in this repo for a good example of a language wrapper.

## Warning

Backbeat is GPL3 software. It is intended for open source rhythm games only. Ensure that you are compatible with this license before statically linking it into your code.

## Usage

C is complex to integrate. We ship prebuilt SDK packages for Linux, macOS, and
Windows in the [GitHub Releases](https://github.com/zkldi/backbeat/releases).
Download and unpack the package for your platform. It contains the
`backbeat.h` header, the static library, a compatible SQLite library,
and CMake/pkg-config metadata.

If you're CMake, the LLM tells me that you should point `CMAKE_PREFIX_PATH` at the unpacked SDK and link the
provided targets:

```cmake
find_package(Backbeat CONFIG REQUIRED)

target_link_libraries(your_game PRIVATE Backbeat::Backbeat Backbeat::SQLite)
```

For example:

```sh
cmake -S . -B build -DCMAKE_PREFIX_PATH=/path/to/backbeat-c-sdk
cmake --build build
```

On Unix, the package also provides a `backbeat.pc` file for use with
`pkg-config`:

```sh
PKG_CONFIG_PATH=/path/to/backbeat-c-sdk/lib/pkgconfig \
  pkg-config --cflags --libs --static backbeat
```

But like all things C, integrating it with your app sucks. Figure out what works for you.
