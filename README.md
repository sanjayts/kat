# kat (cut variation in Rust)
A variation of `cut` command implemented in Rust as part of reading "Command line rust" book. This repo uses the latest version of clap which has a different API compared to the API used in the book.

# Capabilities

This program supports the following capabilities:

```shell
kat 0.1.0
sanjayts

USAGE:
    kat [OPTIONS] [FILE]...

ARGS:
    <FILE>...    [default: -]

OPTIONS:
    -b, --bytes <LIST>         
    -c, --characters <LIST>    
    -d, --delimiter <DELIM>    Use DELIM instead of TAB for delimiter [default: "\t"]
    -f, --fields <LIST>        
    -h, --help                 Print help information
    -V, --version              Print version information
```

This program differs from the original `cut` in a number of ways:

1. It doesn't handle lines with differing delim count. So `1_2_3\n4_5` would cause the second line to error out (TODO)
2. Orders the fields/bytes/chars specified. So `-f 3,2,1` would end up becoming `-f 1,2,3`
3. It doesn't allow more than a single filtering criteria (bytes/chars/fields)
4. It handles quote delimited fields (For a CSV, for e.g. `"10,000 years",abc` line has two fields instead of 3)

# Running Tests

We have a mix of unit and integration tests in our code. The unit tests are in the lib.rs and main.rs file under their respective test mod. We can run unit tests in respective modules using the command:

```shell
cargo test --bin kat #run tests in main
cargo test --lib # run tests in lib.rs
cargo test --test cli # run integration tests in cli.rs
```

# Future enhancements

1. Add support for zero terminated line delimiter (NUL byte)
2. Add support for not printing lines which don't have the delimiter
3. Add support for specifying a custom output delimiter


# Reference

* https://docs.rs/csv/1.0.0-beta.2/csv/struct.Writer.html
* https://man7.org/linux/man-pages/man1/cut.1.html
* For setting up github pipeline for Rust -- https://github.com/actions-rs/meta/tree/master/recipes