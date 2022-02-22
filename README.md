
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

- `--nice-name`/`-n`:
    The name of your project, for purposes of quoting directly into the HTML. This is essentially
    the "brand name" of your project.

- `--output`/`-o`:
    The name of folder to output documentation into.

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
