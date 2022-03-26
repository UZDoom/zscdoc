use itertools::Itertools;
use serde::Serialize;

use crate::{render::render_doc_summary, structures::*};

#[derive(Serialize)]
pub enum SearchResultKind {
    Class,
    Struct,
    Enum,
    Builtin,
    Function,
    Member,
    Constant,
    Enumerator,
}

#[derive(Serialize)]
pub struct SearchResult {
    name_prelude: String,
    name: String,
    link: String,
    desc: String,
    kind: SearchResultKind,
}

#[derive(Serialize)]
pub struct SearchResults {
    results: Vec<SearchResult>,
}

fn summarize(doc_comment: &str) -> String {
    render_doc_summary(doc_comment)
        .map(|x| x.to_string())
        .unwrap_or_else(|| "".to_string())
}

fn collect_class(c: &Class, res: &mut SearchResults) {
    res.results.push(SearchResult {
        name_prelude: "".to_string(),
        name: c.name.to_string(),
        link: format!("class.{}.html", c.name),
        desc: summarize(&c.doc_comment),
        kind: SearchResultKind::Class,
    });
    for co in c.constants.iter() {
        res.results.push(SearchResult {
            name_prelude: format!("{}.", c.name),
            name: co.name.to_string(),
            link: format!("class.{}.html#constant.{}", c.name, co.name),
            desc: summarize(&co.doc_comment),
            kind: SearchResultKind::Constant,
        });
    }
    for mf in [&c.public, &c.protected] {
        for f in mf.functions.iter() {
            res.results.push(SearchResult {
                name_prelude: format!("{}.", c.name),
                name: f.name.to_string(),
                link: format!("class.{}.html#function.{}", c.name, f.name),
                desc: summarize(&f.doc_comment),
                kind: SearchResultKind::Function,
            });
        }
        for m in mf.variables.iter() {
            res.results.push(SearchResult {
                name_prelude: format!("{}.", c.name),
                name: m.name.to_string(),
                link: format!("class.{}.html#member.{}", c.name, m.name),
                desc: summarize(&m.doc_comment),
                kind: SearchResultKind::Member,
            });
        }
    }
    for s in c.inner_structs.iter() {
        collect_struct(s, res);
    }
}

fn collect_struct(s: &Struct, res: &mut SearchResults) {
    let split = s.name.split('.').collect_vec();
    let (last, prelude) = split.split_last().unwrap();
    let name_prelude = if prelude.is_empty() {
        "".to_string()
    } else {
        format!("{}.", prelude.join("."))
    };
    let name = last.to_string();

    res.results.push(SearchResult {
        name_prelude,
        name,
        link: format!("struct.{}.html", s.name),
        desc: summarize(&s.doc_comment),
        kind: SearchResultKind::Struct,
    });
    for co in s.constants.iter() {
        res.results.push(SearchResult {
            name_prelude: format!("{}.", s.name),
            name: co.name.to_string(),
            link: format!("struct.{}.html#constant.{}", s.name, co.name),
            desc: summarize(&co.doc_comment),
            kind: SearchResultKind::Constant,
        });
    }
    for mf in [&s.public, &s.protected] {
        for f in mf.functions.iter() {
            res.results.push(SearchResult {
                name_prelude: format!("{}.", s.name),
                name: f.name.to_string(),
                link: format!("struct.{}.html#function.{}", s.name, s.name),
                desc: summarize(&f.doc_comment),
                kind: SearchResultKind::Function,
            });
        }
        for m in mf.variables.iter() {
            res.results.push(SearchResult {
                name_prelude: format!("{}.", s.name),
                name: m.name.to_string(),
                link: format!("struct.{}.html#member.{}", s.name, m.name),
                desc: summarize(&m.doc_comment),
                kind: SearchResultKind::Member,
            });
        }
    }
    for e in s.inner_enums.iter() {
        collect_enum(e, res);
    }
}

fn collect_builtin(b: &Builtin, res: &mut SearchResults) {
    let split = b.name.split('.').collect_vec();
    let (last, prelude) = split.split_last().unwrap();
    let name_prelude = if prelude.is_empty() {
        "".to_string()
    } else {
        format!("{}.", prelude.join("."))
    };
    let name = last.to_string();

    res.results.push(SearchResult {
        name_prelude,
        name,
        link: format!("builtin.{}.html", b.name),
        desc: summarize(&b.doc_comment),
        kind: SearchResultKind::Builtin,
    });
    for co in b.constants.iter() {
        res.results.push(SearchResult {
            name_prelude: format!("{}.", b.name),
            name: co.name.to_string(),
            link: format!("builtin.{}.html#constant.{}", b.name, co.name),
            desc: summarize(&co.doc_comment),
            kind: SearchResultKind::Constant,
        });
    }
    for f in b.functions.iter() {
        res.results.push(SearchResult {
            name_prelude: format!("{}.", b.name),
            name: f.name.to_string(),
            link: format!("builtin.{}.html#function.{}", b.name, b.name),
            desc: summarize(&f.doc_comment),
            kind: SearchResultKind::Function,
        });
    }
    for m in b.variables.iter() {
        res.results.push(SearchResult {
            name_prelude: format!("{}.", b.name),
            name: m.name.to_string(),
            link: format!("builtin.{}.html#member.{}", b.name, m.name),
            desc: summarize(&m.doc_comment),
            kind: SearchResultKind::Member,
        });
    }
}

fn collect_enum(e: &Enum, res: &mut SearchResults) {
    let split = e.name.split('.').collect_vec();
    let (last, prelude) = split.split_last().unwrap();
    let name_prelude = if prelude.is_empty() {
        "".to_string()
    } else {
        format!("{}.", prelude.join("."))
    };
    let name = last.to_string();

    res.results.push(SearchResult {
        name_prelude,
        name,
        link: format!("enum.{}.html", e.name),
        desc: summarize(&e.doc_comment),
        kind: SearchResultKind::Enum,
    });
    for en in e.enumerators.iter() {
        res.results.push(SearchResult {
            name_prelude: format!("{}.", e.name),
            name: en.name.to_string(),
            link: format!("enum.{}.html#enumerator.{}", e.name, en.name),
            desc: summarize(&en.doc_comment),
            kind: SearchResultKind::Enumerator,
        });
    }
}

pub fn collect_search_results(docs: &Documentation) -> SearchResults {
    let mut res = SearchResults { results: vec![] };
    for c in docs.constants.iter() {
        res.results.push(SearchResult {
            name_prelude: "".to_string(),
            name: c.name.to_string(),
            link: format!("index.html#constant.{}", c.name),
            desc: summarize(&c.doc_comment),
            kind: SearchResultKind::Constant,
        });
    }
    for c in docs.classes.iter() {
        collect_class(c, &mut res);
    }
    for s in docs.structs.iter() {
        collect_struct(s, &mut res);
    }
    for e in docs.enums.iter() {
        collect_enum(e, &mut res);
    }
    for b in docs.builtins.iter() {
        collect_builtin(b, &mut res);
    }
    res
}
