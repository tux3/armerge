
# armerge

You can use armerge to combine multiple static libraries into a single merged `ar` archive.  
Optionally, it is possible to generate a static archive containing a single merged object file, where all non-public symbols are localized (hidden).

This tool requires `ranlib`, `ld`, and `llvm-objcopy` installed on your host system. You may specify a different linker using the `LD` environment variable.

```
USAGE:
    armerge [FLAGS] [OPTIONS] --output <output> [--] [INPUTS]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Print verbose information

OPTIONS:
    -k, --keep-symbols <keep-symbols>...    Accepts regexes of the symbol names to keep global, and localizes the rest
    -o, --output <output>                   Output static library

ARGS:
    <INPUTS>...    Static libraries to merge
```
