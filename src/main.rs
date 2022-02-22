#![recursion_limit = "1024"]

mod structures;

mod document;
mod render;
mod search;

use clap::Parser;
use zscript_parser::{
    filesystem::{Files, GZDoomFolderFileSystem},
    hir::lower::HirLowerer,
    parser_manager::parse_filesystem,
};

#[derive(Parser, Debug)]
#[clap(author, version, about = "zscript documentation generator", long_about = None)]
struct Args {
    #[clap(short, long, help = "Adds a path as a dependency")]
    depends: Vec<String>,

    #[clap(short, long)]
    folder: String,

    #[clap(short, long)]
    nice_name: String,

    #[clap(short, long, help = "The output folder")]
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
        file.write_all(format!("<!DOCTYPE html>{}", docs.render_summary_page()).as_bytes())?;
    }
    for class in docs.classes.iter() {
        let mut file = File::create(path.join(format!("class.{}.html", class.name)))?;
        file.write_all(format!("<!DOCTYPE html>{}", class.render(&docs.name)).as_bytes())?;
        for strukt in class.inner_structs.iter() {
            let mut file = File::create(path.join(format!("struct.{}.html", strukt.name)))?;
            file.write_all(format!("<!DOCTYPE html>{}", strukt.render(&docs.name)).as_bytes())?;
            for enm in strukt.inner_enums.iter() {
                let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
                file.write_all(format!("<!DOCTYPE html>{}", enm.render(&docs.name)).as_bytes())?;
            }
        }
        for enm in class.inner_enums.iter() {
            let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
            file.write_all(format!("<!DOCTYPE html>{}", enm.render(&docs.name)).as_bytes())?;
        }
    }
    for strukt in docs.structs.iter() {
        let mut file = File::create(path.join(format!("struct.{}.html", strukt.name)))?;
        file.write_all(format!("<!DOCTYPE html>{}", strukt.render(&docs.name)).as_bytes())?;
        for enm in strukt.inner_enums.iter() {
            let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
            file.write_all(format!("<!DOCTYPE html>{}", enm.render(&docs.name)).as_bytes())?;
        }
    }
    for enm in docs.enums.iter() {
        let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
        file.write_all(format!("<!DOCTYPE html>{}", enm.render(&docs.name)).as_bytes())?;
    }
    {
        let mut file = File::create(path.join("search.json"))?;
        file.write_all(
            serde_json::to_string(&search::collect_search_results(docs))
                .unwrap()
                .as_bytes(),
        )?;
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    use anyhow::Context;

    let args = Args::parse();

    let filesystem = GZDoomFolderFileSystem::new(args.folder, args.nice_name.clone())
        .context("couldn't load a path")?;
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

    let docs = document::hir_to_doc_structures(args.nice_name, &hir, &files);
    save_docs_to_folder(&args.output, &docs, args.delete_without_confirm).unwrap();

    Ok(())
}
