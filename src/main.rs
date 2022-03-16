#![recursion_limit = "1024"]

mod item;
mod structures;

mod document;
mod render;
mod search;

use crate::{item::ItemProvider, render::render_from_markdown};
use clap::Parser;
use zscript_parser::{
    filesystem::{FileSystem, Files, GZDoomFolderFileSystem},
    hir::lower::HirLowerer,
    parser_manager::{parse_filesystem_config, ParseFileSystemConfig},
};

use crate::item::ToItemProvider;

#[derive(Parser, Debug)]
#[clap(author, version, about = "zscript documentation generator", long_about = None)]
struct Args {
    #[clap(short, long, help = "Path to the folder to document")]
    folder: String,

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

#[derive(serde::Deserialize, Debug)]
struct Config {
    archive: Archive,
    dependency: Option<Vec<Dependency>>,
}

#[derive(serde::Deserialize, Debug)]
struct Dependency {
    path: String,
    url: String,
}

fn base_file_default() -> String {
    "zscript".to_string()
}
#[derive(serde::Deserialize, Debug)]
struct Archive {
    nice_name: String,
    #[serde(default = "base_file_default")]
    base_file: String,
    markdown_file: Option<Vec<MarkdownFile>>,
}

#[derive(serde::Deserialize, Debug)]
struct MarkdownFile {
    filename: String,
    title: String,
}

struct MarkdownFileToRender {
    output_filename: String,
    title: String,
    markdown: String,
}

fn save_docs_to_folder(
    output: &str,
    docs: &structures::Documentation,
    delete_without_confirm: bool,
    item_provider: &ItemProvider,
    favicon: Option<&[u8]>,
    markdown_files: &[MarkdownFileToRender],
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
    for m in markdown_files {
        let mut file = File::create(path.join(&*m.output_filename))?;
        file.write_all(
            format!(
                "<!DOCTYPE html>{}",
                render_from_markdown(
                    &docs.name,
                    &m.title,
                    &m.markdown,
                    &m.output_filename,
                    item_provider,
                )
            )
            .as_bytes(),
        )?;
    }
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

fn get_filesystem(path: &str) -> anyhow::Result<(GZDoomFolderFileSystem, Config)> {
    use anyhow::Context;

    let mut filesystem = GZDoomFolderFileSystem::new(path.to_string(), path.to_string())
        .context("couldn't load a path")?;

    let config_file = filesystem
        .get_file("docs/zscdoc.toml")
        .context("couldn't find zscdoc.toml")?;
    let config_file = config_file.text();

    let config: Config = toml::from_str(config_file).context("config file parsing failed")?;

    let filesystem =
        GZDoomFolderFileSystem::new(path.to_string(), config.archive.nice_name.clone())
            .context("couldn't load a path")?;

    Ok((filesystem, config))
}

fn option_vec_to_vec<T>(v: Option<Vec<T>>) -> Vec<T> {
    v.unwrap_or_default()
}

fn option_slice_to_slice<T>(v: Option<&[T]>) -> &[T] {
    v.unwrap_or(&[])
}

fn collect_dependencies(
    dependencies: &[Dependency],
) -> anyhow::Result<Vec<(GZDoomFolderFileSystem, Config, String)>> {
    use anyhow::Context;
    use std::collections::HashSet;
    fn recurse(
        dependencies: &[Dependency],
        ret: &mut Vec<(GZDoomFolderFileSystem, Config, String)>,
        seen: &mut HashSet<String>,
    ) -> anyhow::Result<()> {
        for d in dependencies.iter() {
            let (filesystem, config) =
                get_filesystem(&d.path).context(format!("loading dependency {}", d.path))?;
            if seen.contains(&config.archive.nice_name) {
                continue;
            }
            seen.insert(config.archive.nice_name.to_string());

            let dependencies = option_slice_to_slice(config.dependency.as_deref());
            recurse(dependencies, ret, seen)?;
            ret.push((filesystem, config, d.url.to_string()));
        }

        Ok(())
    }

    let mut ret = vec![];
    recurse(dependencies, &mut ret, &mut HashSet::new())?;
    Ok(ret)
}

fn main() -> anyhow::Result<()> {
    use anyhow::Context;

    let args = Args::parse();

    let (mut filesystem, config) = get_filesystem(&args.folder).context("loading main archive")?;

    let summary_doc = filesystem
        .get_file("docs/summary.md")
        .map(|s| s.text().to_string())
        .unwrap_or_else(|| "".to_string());

    let favicon = filesystem.get_file("docs/favicon.png");
    let favicon = favicon.as_ref().map(|s| s.data());

    let markdown_files: Result<Vec<_>, _> = option_vec_to_vec(config.archive.markdown_file)
        .iter()
        .map(|m| {
            let output_filename = if let Some(s) = m.filename.strip_suffix(".md") {
                format!("{}.html", s)
            } else {
                anyhow::bail!("file {:?} didn't have extension .md", m.filename);
            };
            let filename_to_get = format!("docs/{}", m.filename);
            let file = filesystem
                .get_file(&filename_to_get)
                .context(format!("file {:?} didn't exist", filename_to_get))?;
            Ok(MarkdownFileToRender {
                output_filename,
                title: m.title.clone(),
                markdown: file.text().to_string(),
            })
        })
        .collect();
    let markdown_files = markdown_files?;

    let depedencies = collect_dependencies(&option_vec_to_vec(config.dependency))?;

    let mut files = Files::default();
    let mut errs = vec![];

    let (mut parsed_vec, dependency_links): (Vec<_>, Vec<_>) =
        itertools::multiunzip(depedencies.into_iter().map(|(f, c, u)| {
            let options = ParseFileSystemConfig {
                root_name: &c.archive.base_file,
            };
            (
                parse_filesystem_config(f, &mut files, &mut errs, &options),
                u,
            )
        }));
    let options = ParseFileSystemConfig {
        root_name: &config.archive.base_file,
    };
    parsed_vec.push(parse_filesystem_config(
        filesystem, &mut files, &mut errs, &options,
    ));
    let hir = HirLowerer::new(&mut errs).lower(parsed_vec).hir;

    if !errs.is_empty() {
        zscript_parser::err::sort_errs(&mut errs);
        let errs_str = zscript_parser::err::repr_errs(&files, &errs);
        eprintln!("{}", errs_str);
        anyhow::bail!("parsing errors occurred; not generating docs");
    }

    let dependencies = structures::Dependencies { dependency_links };

    let item_provider = hir.to_item_provider(&files, &dependencies);
    let docs = document::hir_to_doc_structures(
        summary_doc,
        config.archive.nice_name,
        &hir,
        &files,
        &item_provider,
        &dependencies,
    );
    save_docs_to_folder(
        &args.output,
        &docs,
        args.delete_without_confirm,
        &item_provider,
        favicon,
        &markdown_files,
    )?;

    Ok(())
}
