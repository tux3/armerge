# armerge

[![crates.io](https://img.shields.io/crates/v/armerge.svg)](https://crates.io/crates/armerge)
[![Apache 2 licensed](https://img.shields.io/badge/license-Apache%202-blue)](./LICENSE)
![MSRV](https://img.shields.io/badge/MSRV-1.56-informational)
[![CI](https://github.com/tux3/armerge/workflows/CI/badge.svg)](https://github.com/tux3/armerge/actions?query=workflow%3ACI)

You can use armerge to combine multiple static libraries into a single merged `ar` archive.  
Optionally, armerge can take a list of symbols you want to keep exported, and localizes (hides) the rest.

This allows you to hide your private symbols and the symbols of your dependencies.  
For example, if your static library `libfoo.a` uses OpenSSL's `libcrypto.a`, armerge can create a single
`libfoo_merged.a` that combines both, but where all the OpenSSL symbols are hidden
and only public `libfoo` symbols of your choice are exported.

## Usage

Example command to merge `libfoo.a` and `libcrypto.a`, keeping only symbols starting with `libfoo_` public:

`armerge --keep-symbols '^libfoo_' --output libfoo_merged.a libfoo.a libcrypto.a`

Options and usage:

```
USAGE:
    armerge [FLAGS] [OPTIONS] --output <output> [--] [INPUTS]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Print verbose information

OPTIONS:
    -k, --keep-symbols <keep-symbols>...     Accepts regexes of the symbol names to keep global, and localizes the rest
    -r, --remove-symbols <remove-symbols>... Accepts regexes of the symbol names to hide, and keep the rest global
    -o, --output <output>                    Output static library

ARGS:
    <INPUTS>...    Static libraries to merge
```

## Platform support

Linux/Android and macOS/iOS `ar` archives are supported, using their respective host toolchain.  
When localizing symbols (`-k` option), only archives containing ELF or Mach-O objects are supported
(and in this case the output archive will contain a single relocatable object `merged.o`).

This tool requires `ranlib`, `ld`, and `llvm-objcopy` to handle Linux static libraries.   
For macOS libraries, `libtool` and the Apple `ld` are used instead.

You may specify a different linker using the `LD` environment variable, and linker flags with `ARMERGE_LDFLAGS`.  
You may specify a different objcopy implementation with the `OBJCOPY` env var, and a different ranlib with `RANLIB`.

You can use armerge to handle Linux/Android archives on a macOS host if the right toolchain is installed.
(i.e. you may need to set `LD`, `OBJCOPY`, and `RANLIB` to point to the Android NDK, or to some other toolchain).

## Principle of operation

### Merging static libraries

When you're not interested in controlling which symbols are exported by your static libraries, merging them is surprisingly simple.
On all major platforms, a static library is really an archive (like a `.zip` file, but in `ar` format).  
(In fact, some projects sometimes handle merging static libraries with ad-hoc shell scripts that call the `ar` tool.)

Essentially all armerge has to do to combine multiple `.a` files into a single one is to extract the object files inside
(being careful not to overwrite different files with the same name, which is allowed *even in a single archive*,
something shell scripts often forget to handle), and add them all together in a new `ar` archive.

For performance reasons, `ar` archive usually also have an index, so we take care to recreate it when merging.

### Controlling exported symbols

When you create a dynamic library, you can choose which set of symbols you want to export and which you want to keep internal.  
This allows keeping dependencies and implementation details private, and prevents hard to debug symbol clashes.  
For example, a `libfoo.so` library could bundle a copy of OpenSSL without problem, but only as long as it doesn't export those symbols.
Hiding symbols is especially important on Android (which helpfully loads its own version of some popular dynamic libraries, like OpenSSL),
although clashes can happen on any platform.

Unfortunately the native C and C++ toolchains do not expose any options to hide symbols in static libraries.  
This is because — unlike dynamic libraries — static archives are not a coherent whole but just a bag of object files.
There is no concept of export control at the static library level, only at the object level.  
So if symbols are naively localized at the object level, the static library would simply fail to link when you use it,
because each object would now fiercely keep its own symbols private from other objects, *even in the same library*.

The solution is to do the same thing that dynamic libraries do: have it consist of a single object file.
So in addition to merging multiple archive files into a single one, when you ask armerge to localize symbols it will
pass all the object files in input libraries to the linker and create a single pre-linked object file, called a relocatable object.

This results in a static library (for instance `libfoo.a`) that contains a single `merged.o` object.  
Since all the input objects have already been pre-linked together, now there is a single symbol list for the whole library
(like dynamic libraries have), and so we can ask the toolchain to localize symbols for us without any problems.

In this process, armerge really delegates most of the work to the linker and utilities like `objcopy`.  
The features required to control symbol export in static libraries is actually the same mechanism that is used for dynamic libraries
(both are really just object files), so your system toolchain had the support for this all along.

But because linkers traditionally output static libraries containing multiple objects instead of a single pre-linked object,
they've created an historical asymmetry where hiding symbols in dynamic libraries has always been easy,
while hiding symbols in static libraries has never been supported by the toolchain.

So, armerge first corrects the asymmetry by asking the linker to pre-link the objects, and only then asks it to hide symbols. 
