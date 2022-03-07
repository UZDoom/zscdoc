# zscdoc
### A GZDoom ZScript documentation generation tool

`zscdoc` is a tool for generating documentation of ZScript code, similar to tools
for other languages such as `rustdoc` or `doxygen`.

It takes in as input "doc-commented" ZScript code and outputs a folder of HTML that can be hosted
without needing anything beyond a basic webserver, and as such can be put on a service like
GitHub/GitLab pages.

## Usage

`zscdoc` takes the following command line arguments:

- `--folder`/`-f`:
    A path containing ZScript code (note that a file named `zscript` (extension-agnostic)
    is currently expected in the folder to know what to include)

- `--output`/`-o`:
    The name of folder to output documentation into.

The following files are used if found inside your archive:

- `docs/summary.md`:
    Markdown that will be added to the summary page. You can use this to give a documentation
    overview of your project as a whole, including linking to other markdown files defined in the
    configuration file.

- `docs/zscdoc.toml`:
    A `TOML`-formatted configuration file that allows you to lay out specific project structure.
    Example configuration file:

    ```toml
    [archive]
    nice_name = "ZForms"

    [[archive.markdown_file]]
    filename = "test.md"
    title = "Test"
    ```

    In the above example, `nice_name` is the name that will be quoted into the documentation when
    relevant.

    An `[[archive.markdown_file]]` block allows you to put a markdown file into your documentation.
    Note that files added like this will have their `.md` extension replaced with `.html` in the
    generated docs. You can use multiple of these to add multiple `.md` files. Note that if a
    filename would clash with a generated documentation file, the generated documentation wins. To
    avoid this, don't call files things like `index.md` or `class.Something.md`, for example.

- `docs/favicon.png`:
    The favicon for the page to use. As of right now if one isn't given your page will be missing a
    favicon.

## Installing

Currently the only way to install the software is via cloning the repository and building it.

Building in release mode (which is probably what you want unless you intend to help with
development) is pretty simple. You must have the rust toolchain (i.e. `cargo`) installed, and
`npm`, the Node Package Manager. From there you can simply run `cargo install --path .` from the
repository and it'll be installed to your Cargo `PATH` under the binary name `zscdoc`.

Alternatively, you can use `cargo build --release` and pull the binary out from the
`target/release` folder. Or just run it via `cargo run --release`.

## Building in development mode

Building and running in development mode works slightly differently to help with development speed.
Notably, you must run the `npm` commands manually, and then the program must be run from the folder
containing `Cargo.toml`.

This is because in development mode, the program reads the static JavaScript and CSS files at runtime
and doesn't include them in the binary. Then, to avoid pointlessly recompiling the Rust parts of the program
every time the JS/CSS parts are changed, `npm` must be run manually. See the `Justfile` in the repo for
a simple example of how to do this using the included `zforms` example in `test_data/`.
