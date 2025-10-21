use clap::{ArgEnum, ArgGroup, Parser};

#[derive(ArgEnum, Debug, Clone, Copy)]
pub enum CoverageLevel {
    Percentage,
    Breakdown,
    Verbose,
}

#[derive(Parser, Debug)]
#[clap(author, version, about = "zscript documentation generator", long_about = None)]
#[clap(group(ArgGroup::new("mode").required(true)))]
pub struct Args {
    #[clap(short, long, help = "Path to the folder to document")]
    pub folder: String,

    #[clap(short, long, help = "Path for the output folder", group = "mode")]
    pub output: Option<String>,

    #[clap(
        long,
        arg_enum,
        help = "Shows the doc coverage in one of a few formats",
        group = "mode"
    )]
    pub coverage: Option<CoverageLevel>,

    #[clap(
        long,
        help = "Deletes the target folder without confirmation. Best kept off in most cases."
    )]
    pub delete_without_confirm: bool,

    #[clap(
        long,
        help = "The base template for URLs in links in the documentation - include the string `<version>` to have it substituted with the value from`--version`"
    )]
    pub base_url: Option<String>,

    #[clap(
        long,
        help = r#"A JSON array of objects like {"url_part": "v4", "nice_name": "4.0 (Latest)", latest: true}"#
    )]
    pub versions: Option<String>,

    #[clap(
        long,
        help = "The version currently being documented - this should match the `url_part` for one item in `--versions`"
    )]
    pub target_version: Option<String>,

    #[clap(long, help = "The canonical domain you're hosting the docs on")]
    pub canonical_domain: Option<String>,
}
