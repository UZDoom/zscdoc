#![recursion_limit = "1024"]
#![allow(clippy::too_many_arguments)]

mod item;
mod structures;

mod builtin;
mod cli;
mod coverage;
mod document;
mod git;
mod render;
mod search;

use crate::{
    builtin::BuiltinTypeFromFile, cli::*, coverage::coverage_breakdown, item::ItemProvider,
    render::render_from_markdown, structures::BaseUrl,
};
use clap::Parser;
use itertools::Itertools;
use zscript_parser::{
    err::ToDisplayedErrors,
    filesystem::{File, FileSystem, Files, GZDoomFolderFileSystem},
    hir::lower::HirLowerer,
    parser_manager::{parse_filesystem_config, ParseFileSystemConfig},
};

use crate::item::ToItemProvider;

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
#[serde(untagged)]
enum DependencyPathKind {
    Path {
        path: String,
    },
    Git {
        git: String,
        refname: String,
        #[serde(default = "String::new")]
        base: String,
    },
}

#[derive(serde::Deserialize, Debug)]
struct Dependency {
    #[serde(flatten)]
    find_at: DependencyPathKind,
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
    #[serde(alias = "markdown_file")]
    markdown_files: Option<Vec<MarkdownFile>>,
    #[serde(alias = "copy_file")]
    copy_files: Option<Vec<CopyFile>>,
    #[serde(alias = "builtin")]
    builtins: Option<Vec<String>>,
    #[serde(default = "String::new")]
    base_url: String,
    #[serde(default)]
    document_globals: bool,
}

#[derive(serde::Deserialize, Debug)]
struct MarkdownFile {
    filename: String,
    title: String,
}

#[derive(serde::Deserialize)]
pub struct VersionItem {
    url_part: String,
    nice_name: String,
    no_index: bool,
    title_suffix: String,
    #[expect(unused)]
    latest: bool,
}

#[derive(serde::Deserialize)]
pub struct VersionInfo {
    current: String,
    no_index: bool,
    title_suffix: String,
    versions: Vec<VersionItem>,
}

#[derive(serde::Deserialize, Debug)]
struct CopyFile {
    filename: String,
}

struct MarkdownFileToRender {
    output_filename: String,
    title: String,
    markdown: String,
}

struct CopyFileToRender {
    output_filename: String,
    bytes: Vec<u8>,
}

fn save_docs_to_folder(
    output: &str,
    docs: &structures::Documentation,
    delete_without_confirm: bool,
    item_provider: &ItemProvider,
    favicon: Option<&[u8]>,
    markdown_files: &[MarkdownFileToRender],
    copy_files: &[CopyFileToRender],
    base: &BaseUrl,
    version_info: Option<VersionInfo>,
    canonical_domain: Option<String>,
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
    for m in copy_files {
        let mut file = File::create(path.join(&*m.output_filename))?;
        file.write_all(&m.bytes)?;
    }
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
                    base,
                    version_info.as_ref(),
                )
            )
            .as_bytes(),
        )?;
    }
    for asset_path in Assets::iter() {
        let mut file = File::create(path.join(&*asset_path))?;
        file.write_all(&Assets::get(&asset_path).unwrap().data)?;
    }
    {
        let mut file = File::create(path.join("index.html"))?;
        file.write_all(
            format!(
                "<!DOCTYPE html>{}",
                docs.render_summary_page(
                    item_provider,
                    base,
                    version_info.as_ref(),
                    canonical_domain.as_deref(),
                )
            )
            .as_bytes(),
        )?;
    }
    for class in docs.classes.iter() {
        let mut file = File::create(path.join(format!("class.{}.html", class.name)))?;
        file.write_all(
            format!(
                "<!DOCTYPE html>{}",
                class.render(&docs.name, item_provider, base, version_info.as_ref())
            )
            .as_bytes(),
        )?;
        for strukt in class.inner_structs.iter() {
            let mut file = File::create(path.join(format!("struct.{}.html", strukt.name)))?;
            file.write_all(
                format!(
                    "<!DOCTYPE html>{}",
                    strukt.render(&docs.name, item_provider, base, version_info.as_ref())
                )
                .as_bytes(),
            )?;
            for enm in strukt.inner_enums.iter() {
                let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
                file.write_all(
                    format!(
                        "<!DOCTYPE html>{}",
                        enm.render(&docs.name, item_provider, base, version_info.as_ref())
                    )
                    .as_bytes(),
                )?;
            }
        }
        for enm in class.inner_enums.iter() {
            let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
            file.write_all(
                format!(
                    "<!DOCTYPE html>{}",
                    enm.render(&docs.name, item_provider, base, version_info.as_ref())
                )
                .as_bytes(),
            )?;
        }
    }
    for strukt in docs.structs.iter() {
        let mut file = File::create(path.join(format!("struct.{}.html", strukt.name)))?;
        file.write_all(
            format!(
                "<!DOCTYPE html>{}",
                strukt.render(&docs.name, item_provider, base, version_info.as_ref())
            )
            .as_bytes(),
        )?;
        for enm in strukt.inner_enums.iter() {
            let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
            file.write_all(
                format!(
                    "<!DOCTYPE html>{}",
                    enm.render(&docs.name, item_provider, base, version_info.as_ref())
                )
                .as_bytes(),
            )?;
        }
    }
    for enm in docs.enums.iter() {
        let mut file = File::create(path.join(format!("enum.{}.html", enm.name)))?;
        file.write_all(
            format!(
                "<!DOCTYPE html>{}",
                enm.render(&docs.name, item_provider, base, version_info.as_ref())
            )
            .as_bytes(),
        )?;
    }
    for builtin in docs.builtins.iter() {
        let mut file = File::create(path.join(format!("builtin.{}.html", builtin.name)))?;
        file.write_all(
            format!(
                "<!DOCTYPE html>{}",
                builtin.render(&docs.name, item_provider, base, version_info.as_ref())
            )
            .as_bytes(),
        )?;
    }
    {
        let mut file = File::create(path.join("search.json"))?;
        file.write_all(
            serde_json::to_string(&search::collect_search_results(docs, item_provider, base))
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

fn get_filesystem(path: &str) -> anyhow::Result<(GZDoomFolderFileSystem, Config, Vec<File>)> {
    use anyhow::Context;

    let mut filesystem = GZDoomFolderFileSystem::new(path.to_string(), path.to_string())
        .context("couldn't load a path")?;

    let config_file = filesystem
        .get_file("docs/zscdoc.toml")
        .context("couldn't find zscdoc.toml")?;
    let config_file = config_file.text();

    let config: Config = toml::from_str(config_file).context("config file parsing failed")?;

    let mut filesystem =
        GZDoomFolderFileSystem::new(path.to_string(), config.archive.nice_name.clone())
            .context("couldn't load a path")?;

    let builtin_files: Result<Vec<_>, anyhow::Error> =
        option_slice_to_slice(config.archive.builtins.as_deref())
            .iter()
            .map(|s| {
                let filename_to_get = format!("docs/{}", s);
                let file = filesystem
                    .get_file(&filename_to_get)
                    .context(format!("file {:?} didn't exist", filename_to_get))?;
                Ok(file)
            })
            .collect();
    let builtin_files = builtin_files?;

    Ok((filesystem, config, builtin_files))
}

pub fn option_vec_to_vec<T>(v: Option<Vec<T>>) -> Vec<T> {
    v.unwrap_or_default()
}

pub fn option_slice_to_slice<T>(v: Option<&[T]>) -> &[T] {
    v.unwrap_or(&[])
}

fn get_builtins(files: &[File]) -> anyhow::Result<impl Iterator<Item = BuiltinTypeFromFile> + '_> {
    use anyhow::Context;

    let r: anyhow::Result<Vec<_>> = files
        .iter()
        .map(|f| {
            let mut builtin: BuiltinTypeFromFile =
                toml::from_str(f.text()).context("builtin file parsing failed")?;
            builtin.filename = f.filename().to_string();
            Ok(builtin)
        })
        .collect();
    r.map(|r| r.into_iter())
}

struct CollectedDependency {
    filesystem: GZDoomFolderFileSystem,
    config: Config,
    url: String,
    builtins: Vec<BuiltinTypeFromFile>,
}

fn collect_dependencies(
    dependencies: &[Dependency],
    base_path: &str,
) -> anyhow::Result<Vec<CollectedDependency>> {
    use anyhow::Context;
    use std::collections::HashSet;
    fn recurse(
        dependencies: &[Dependency],
        ret: &mut Vec<CollectedDependency>,
        seen: &mut HashSet<String>,
        base_path: &std::path::Path,
    ) -> anyhow::Result<()> {
        for d in dependencies.iter() {
            let dep_path = match &d.find_at {
                DependencyPathKind::Path { path } => base_path.join(path),
                DependencyPathKind::Git { git, refname, base } => {
                    eprintln!(
                        "Cloning git repository for dependency: {}, ref {}...",
                        git, refname
                    );
                    git::clone_git(git, refname)
                        .context("git cloning failed")?
                        .join(base)
                }
            };
            let dep_path_str = dep_path.to_str().context("paths must be UTF-8")?;
            let (filesystem, config, builtin_files) = get_filesystem(dep_path_str)
                .context(format!("loading dependency path {:?}", dep_path))?;
            if seen.contains(&config.archive.nice_name) {
                continue;
            }
            seen.insert(config.archive.nice_name.to_string());

            let dependencies = option_slice_to_slice(config.dependency.as_deref());
            recurse(dependencies, ret, seen, &dep_path)?;
            let builtins = get_builtins(&builtin_files)?.collect_vec();
            ret.push(CollectedDependency {
                filesystem,
                config,
                url: d.url.to_string(),
                builtins,
            });
        }

        Ok(())
    }

    let mut ret = vec![];
    recurse(
        dependencies,
        &mut ret,
        &mut HashSet::new(),
        std::path::Path::new(base_path),
    )?;
    Ok(ret)
}

fn main() -> anyhow::Result<()> {
    use anyhow::Context;

    let args = Args::parse();

    let mut files = Files::default();

    let (mut filesystem, config, builtin_files) =
        get_filesystem(&args.folder).context("loading main archive")?;

    let summary_doc = filesystem
        .get_file("docs/summary.md")
        .map(|s| s.text().to_string())
        .unwrap_or_else(|| "".to_string());

    let favicon = filesystem.get_file("docs/favicon.png");
    let favicon = favicon.as_ref().map(|s| s.data());

    let markdown_files: Result<Vec<_>, _> = option_vec_to_vec(config.archive.markdown_files)
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

    let copy_files: Result<Vec<_>, anyhow::Error> = option_vec_to_vec(config.archive.copy_files)
        .iter()
        .map(|m| {
            let output_filename = m.filename.clone();
            let filename_to_get = format!("docs/{}", m.filename);
            let file = filesystem
                .get_file(&filename_to_get)
                .context(format!("file {:?} didn't exist", filename_to_get))?;
            Ok(CopyFileToRender {
                output_filename,
                bytes: file.data().to_vec(),
            })
        })
        .collect();
    let copy_files = copy_files?;

    let depedencies = collect_dependencies(&option_vec_to_vec(config.dependency), &args.folder)?;

    let mut errs = vec![];

    eprintln!("Parsing ZScript code...");
    let (mut parsed_vec, dependency_links, mut builtins): (Vec<_>, Vec<_>, Vec<_>) =
        itertools::multiunzip(depedencies.into_iter().map(|d| {
            let CollectedDependency {
                filesystem,
                config,
                url,
                builtins,
            } = d;
            let options = ParseFileSystemConfig {
                root_name: &config.archive.base_file,
            };
            (
                parse_filesystem_config(filesystem, &mut files, &mut errs, &options),
                crate::structures::Dependency { link: url },
                builtins,
            )
        }));
    builtins.push(get_builtins(&builtin_files)?.collect_vec());
    let builtins = builtins;

    let options = ParseFileSystemConfig {
        root_name: &config.archive.base_file,
    };
    parsed_vec.push(parse_filesystem_config(
        filesystem, &mut files, &mut errs, &options,
    ));
    let hir = HirLowerer::new(&mut errs).lower(parsed_vec).hir;

    if !errs.is_empty() {
        return Err(anyhow::anyhow!(errs.to_displayed_errors(&files)))
            .context("failed to parse ZScript source");
    }

    let dependencies = structures::Dependencies { dependency_links };

    let mut item_provider = hir.to_item_provider(&files, &dependencies);

    let mut builtins = builtins
        .into_iter()
        .map(|b| {
            b.into_iter()
                .map(|b| b.produce(&mut files))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|x| x.to_displayed_errors(&files))?;

    for b in builtins.iter_mut() {
        for b in b.iter_mut() {
            b.extend_with_uses_things_from(&hir)?;
        }
    }

    item_provider.add_builtins(&builtins, &files, &dependencies);
    let item_provider = item_provider;

    let builtins = builtins
        .pop()
        .unwrap()
        .into_iter()
        .map(|b| b.produce(&mut files, &item_provider))
        .collect_vec();

    eprintln!("Generating documentation structures...");
    let docs = document::hir_to_doc_structures(
        summary_doc,
        &config.archive.nice_name,
        &hir,
        &files,
        &item_provider,
        &dependencies,
        builtins,
        config.archive.document_globals,
    );

    let base_url = args.base_url.unwrap_or(config.archive.base_url);

    let versions: Option<Vec<VersionItem>> = args
        .versions
        .as_ref()
        .map(|v| serde_json::from_str(v))
        .transpose()?;

    let version_info = if versions.is_some() || args.target_version.is_some() {
        let Some(versions) = versions else {
            anyhow::bail!("`--versions` must be present if `--target-version` is")
        };
        let Some(version) = args.target_version else {
            anyhow::bail!("`--target-version` must be present if `--version-map` is")
        };
        let Some(current_version_item) = versions.iter().find(|v| v.url_part == version) else {
            anyhow::bail!("`--target-version` not found in `--version")
        };
        let no_index = current_version_item.no_index;
        let title_suffix = if current_version_item.title_suffix.is_empty() {
            "".to_string()
        } else {
            format!(" {}", current_version_item.title_suffix)
        };
        if !base_url.contains("<version>") {
            anyhow::bail!(
                "`--base-url` must contain the string <version> when version support is active"
            )
        }
        Some(VersionInfo {
            current: version,
            versions,
            no_index,
            title_suffix,
        })
    } else {
        None
    };

    let base_url = BaseUrl {
        template: base_url.to_string(),
        filled: base_url.replace(
            "<version>",
            &version_info
                .as_ref()
                .map(|v| v.current.clone())
                .unwrap_or_else(|| "<version>".to_string()),
        ),
    };

    if let Some(c) = args.coverage {
        let breakdown = coverage_breakdown(
            docs.coverage(&config.archive.nice_name, &files)
                .collect_vec(),
        );
        breakdown.show(c);
    } else {
        let out = args.output.unwrap();
        save_docs_to_folder(
            &out,
            &docs,
            args.delete_without_confirm,
            &item_provider,
            favicon,
            &markdown_files,
            &copy_files,
            &base_url,
            version_info,
            args.canonical_domain,
        )?;
        eprintln!("Documentation written to {}!", out);
    }

    Ok(())
}
