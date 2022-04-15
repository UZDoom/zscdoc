# zscdoc

### A GZDoom ZScript documentation generation tool

`zscdoc` is a tool for generating documentation of ZScript code, similar to
tools for other languages such as `rustdoc` or `doxygen`.

It takes in as input "doc-commented" ZScript code and outputs a folder of HTML
that can be hosted without needing anything beyond a basic webserver, and as
such can be put on a service like GitHub/GitLab pages.

## Usage

`zscdoc` is operated with a command line interface. See `zscdoc --help` for
more details.

A configuration file named `docs/zscdoc.toml` in a documented archive is
required, with the following format:

```toml
[archive]
nice_name = "ZForms"
base_file = "ZForms/include"
base_url = ""

[[archive.markdown_file]]
filename = "test.md"
title = "Test"

[[archive.copy_file]]
filename = "test.png"

[[dependency]]
path = "../gzdoom_pk3"
url = "http://localhost:8081"
```

This example shows all of the options available, some of which are optional.

`archive.base_url` is optional. It defaults to `""`, and is prepended onto all
generated URLs in the documentation, for use in systems like GitHub/GitLab
pages which prepend your site with a repository name.

`archive.nice_name` is the name that will be quoted into the documentation when
relevant.

`archive.base_file` is optional (defaulting to `"zscript"`) and determines what will be
treated as the starting point for the archive. This is to avoid having to
create a `zscript` file that simply includes some other actually-intended
include file.

An `[[archive.markdown_file]]` block allows you to put a markdown file into
your documentation. Note that files added like this will have their `.md`
extension replaced with `.html` in the generated docs. You can use multiple of
these to add multiple `.md` files. Note that if a filename would clash with a
generated documentation file, the generated documentation wins. To avoid this,
don't call files things like `index.md` or `class.Something.md`, for example.

An `[[archive.copy_file]]` block copies a file directly from the `docs` folder
into the output documentation without any processing. This can be used for
adding images or other assets to the docs. Same stuff from `markdown_file`
about these files being overridden by generated assets applies.

A `[[dependency]]` block can be used to add a linked dependency to your
documentation. This will make it so that types outside of your own library can
be resolved by linking to another URL which is assumed to have been generated
with the same version of `zscdoc`.

The following special files are used if found inside your archive:

- `docs/summary.md`:
    Markdown that will be added to the summary page. You can use this to give a
    documentation overview of your project as a whole, including linking to
    other markdown files defined in the configuration file.

- `docs/favicon.png`:
    The favicon for the page to use. As of right now if one isn't given your
    page will be missing a favicon.

## Installing

Currently the only way to install the software is via building it, which is
pretty simple due to Rust's toolchain. You must have the Rust toolchain (i.e.
`cargo`) installed, and `npm`, the Node Package Manager. From there you can
simply run `cargo install --git https://gitlab.com/Gutawer/zscdoc` and it'll be
installed to your Cargo `PATH` under the binary name `zscdoc`.

Alternatively, you can clone the repository and install it via `cargo install
--path .`. You can also clone the repository and use `cargo build --release`
and pull the binary out from the `target/release` folder, or just run it via
`cargo run --release`.

## Building in development mode

Building and running in development mode works slightly differently to help
with development speed. Notably, you must run the `npm` commands manually, and
then the program must be run from the folder containing `Cargo.toml`.

This is because in development mode, the program reads the static JavaScript
and CSS files at runtime and doesn't include them in the binary. Then, to avoid
pointlessly recompiling the Rust parts of the program every time the JS/CSS
parts are changed, `npm` must be run manually. See the `Justfile` in the repo
for a simple example of how to do this using the included `zforms` example in
`test_data/`. Note that you must install the node dependencies via `npm
install` first.
