#![recursion_limit = "1024"]

mod item;
mod structures;

mod document;
mod render;
mod search;

use crate::item::ItemProvider;
use clap::Parser;
use zscript_parser::{
    filesystem::{FileSystem, Files, GZDoomFolderFileSystem},
    hir::lower::HirLowerer,
    parser_manager::parse_filesystem,
};

use crate::item::ToItemProvider;

#[derive(Parser, Debug)]
#[clap(author, version, about = "zscript documentation generator", long_about = None)]
struct Args {
    //#[clap(short, long, help = "Adds a path as a dependency")]
    //depends: Vec<String>,
    #[clap(short, long, help = "Path to the folder to document")]
    folder: String,

    #[clap(
        short,
        long,
        help = "The name of the library to use for quoting into the documentation"
    )]
    nice_name: String,

    #[clap(short, long, help = "Path for the output folder")]
    output: String,

    #[clap(
        long,
        help = "Deletes the target folder without confirmation. Best kept off in most cases."
    )]
    delete_without_confirm: bool,
}

#[cfg(not(debug_assertions))]
#[derive(rust_embed::RustEmbed)]
#[folder = "$OUT_DIR/web_stuff/dist"]
struct Assets;

#[cfg(debug_assertions)]
#[derive(rust_embed::RustEmbed)]
#[folder = "web_stuff/dist"]
struct Assets;

fn save_docs_to_folder(
    output: &str,
    docs: &structures::Documentation,
    delete_without_confirm: bool,
    item_provider: &ItemProvider,
    favicon: Option<&[u8]>,
) -> anyhow::Result<()> {
    use std::fs::*;
    use std::io::*;
    let path = std::path::PathBuf::from(output);
    if path.exists() {
        if delete_without_confirm {
            remove_dir_all(&path)?;
        } else {
            print!("Path {:?} exists. Delete (yN)? ", path);
            stdout().flush().unwrap();
            let mut buffer = String::new();
            stdin().read_line(&mut buffer)?;
            if buffer == "y\n" || buffer == "Y\n" {
                remove_dir_all(&path)?;
            } else {
                anyhow::bail!("Path not deleted.");
            }
        }
    }
    create_dir(&path)?;
    for asset_path in Assets::iter() {
        let mut file = File::create(path.join(&*asset_path))?;
        file.write_all(&Assets::get(&*asset_path).unwrap().data)?;
    }
    {
        let mut file = File::create(path.join("index.html"))?;
        file.write_all(
            format!("<!DOCTYPE html>{}", docs.render_summary_page(item_provider)).as_bytes(),
        )?;
    }
    for class in docs.classes.iter() {
        let mut file = File::create(path.join(format!("class.{}.html", class.name)))?;
        file.write_all(
            format!("<!DOCTYPE html>{}", class.render(&docs.name, item_provider)).as_bytes(),
        )?;
        for strukt in class.inner_structs.iter() {
            let mut file = File::create(path.join(format!("struct.{}.html", strukt.name)))?;
            file.write_all(
                format!(
                    "<!DOCTYPE html>{}",
                    strukt.render(&docs.name, item_provider)
                )
                .as_bytes(),
            )?;
            for enm in strukt.inner_enums.iter() {
                let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
                file.write_all(
                    format!("<!DOCTYPE html>{}", enm.render(&docs.name, item_provider)).as_bytes(),
                )?;
            }
        }
        for enm in class.inner_enums.iter() {
            let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
            file.write_all(
                format!("<!DOCTYPE html>{}", enm.render(&docs.name, item_provider)).as_bytes(),
            )?;
        }
    }
    for strukt in docs.structs.iter() {
        let mut file = File::create(path.join(format!("struct.{}.html", strukt.name)))?;
        file.write_all(
            format!(
                "<!DOCTYPE html>{}",
                strukt.render(&docs.name, item_provider)
            )
            .as_bytes(),
        )?;
        for enm in strukt.inner_enums.iter() {
            let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
            file.write_all(
                format!("<!DOCTYPE html>{}", enm.render(&docs.name, item_provider)).as_bytes(),
            )?;
        }
    }
    for enm in docs.enums.iter() {
        let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
        file.write_all(
            format!("<!DOCTYPE html>{}", enm.render(&docs.name, item_provider)).as_bytes(),
        )?;
    }
    {
        let mut file = File::create(path.join("search.json"))?;
        file.write_all(
            serde_json::to_string(&search::collect_search_results(docs))
                .unwrap()
                .as_bytes(),
        )?;
    }
    if let Some(f) = favicon {
        let mut file = File::create(path.join("favicon.png"))?;
        file.write_all(f)?;
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    use anyhow::Context;

    let args = Args::parse();

    let mut filesystem = GZDoomFolderFileSystem::new(args.folder, args.nice_name.clone())
        .context("couldn't load a path")?;

    let summary_doc = filesystem
        .get_file("docs/summary.md")
        .map(|s| s.text().to_string())
        .unwrap_or_else(|| "".to_string());

    let favicon = filesystem.get_file("docs/favicon.png");
    let favicon = favicon.as_ref().map(|s| s.data());

    let mut files = Files::default();
    let mut errs = vec![];
    let parsed = parse_filesystem(filesystem, &mut files, &mut errs);
    let hir = HirLowerer::new(&mut errs).lower(vec![parsed]).hir;

    if !errs.is_empty() {
        zscript_parser::err::sort_errs(&mut errs);
        let errs_str = zscript_parser::err::repr_errs(&files, &errs);
        eprintln!("{}", errs_str);
        anyhow::bail!("parsing errors occurred; not generating docs");
    }

    let item_provider = hir.to_item_provider(&files);
    let docs =
        document::hir_to_doc_structures(summary_doc, args.nice_name, &hir, &files, &item_provider);
    save_docs_to_folder(
        &args.output,
        &docs,
        args.delete_without_confirm,
        &item_provider,
        favicon,
    )
    .unwrap();

    Ok(())
}
