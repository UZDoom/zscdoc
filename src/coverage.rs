use crate::structures::*;
use itertools::Itertools;
use zscript_parser::filesystem::Files;

pub use crate::cli::CoverageLevel;

#[derive(Debug, Clone)]
pub enum CoverageKind {
    Summary,
    Struct,
    Class,
    Enum,
    Builtin,
    Function,
    Member,
    Enumerator,
    Constant,
    Property,
    Flag,
}

#[derive(Debug, Clone)]
pub struct CoverageItem {
    pub covered: bool,
    pub kind: CoverageKind,
    pub filename: String,
    pub path: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CoverageBreakdown {
    pub proportion: f32,
    pub all: Vec<CoverageItem>,
    pub not_covered: Vec<CoverageItem>,
}

impl CoverageBreakdown {
    fn print_rough_breakdown(&self) {
        use prettytable::*;

        struct FileBreakdown {
            filename: String,
            count: String,
            percent: String,
        }
        let mut v = self
            .all
            .iter()
            .into_group_map_by(|x| x.filename.clone())
            .iter()
            .map(|(k, v)| {
                let doc_count = v.iter().filter(|x| x.covered).count();
                FileBreakdown {
                    filename: k.clone(),
                    count: format!("{}", doc_count),
                    percent: format!("{}%", doc_count as f32 / v.len() as f32 * 100.0),
                }
            })
            .collect_vec();
        v.sort_unstable_by(|x, y| x.filename.cmp(&y.filename));
        let v = v;

        let mut table = Table::new();
        table.add_row(row![
            b->"Filename",
            b->"Documented Count",
            b->"Documented Percentage",
        ]);

        for r in v.iter() {
            table.add_row(row![r.filename, r.count, r.percent]);
        }

        table.printstd();
    }

    fn print_full_breakdown(&self) {
        use prettytable::*;

        let mut table = Table::new();

        table.add_row(row![
            b->"Filename",
            b->"Type",
            b->"Path",
        ]);

        for c in self.not_covered.iter() {
            table.add_row(row![c.filename, format!("{:?}", c.kind), c.path.join(".")]);
        }

        table.printstd();
    }

    pub fn show(&self, level: CoverageLevel) {
        print!("Doc coverage: {}%", self.proportion * 100.0);
        match level {
            CoverageLevel::Percentage => {}
            CoverageLevel::Breakdown => {
                println!();
                println!("\nBreakdown:");
                self.print_rough_breakdown();
                println!();
            }
            CoverageLevel::Verbose => {
                println!();
                println!("\nBreakdown:");
                self.print_rough_breakdown();
                println!("\nNot covered:");
                self.print_full_breakdown();
                println!();
            }
        }
    }
}

pub fn coverage_breakdown(i: impl IntoIterator<Item = CoverageItem>) -> CoverageBreakdown {
    let all = i.into_iter().collect_vec();
    let not_covered = all.iter().filter(|x| !x.covered).cloned().collect_vec();
    let proportion = 1.0 - not_covered.len() as f32 / all.len() as f32;
    CoverageBreakdown {
        proportion,
        all,
        not_covered,
    }
}

fn context_with(context: &[String], with: &str) -> Vec<String> {
    let mut v = context.to_vec();
    v.push(with.to_string());
    v
}

impl Documentation {
    pub fn coverage<'a>(
        &'a self,
        base: &str,
        files: &'a Files,
    ) -> impl Iterator<Item = CoverageItem> + 'a {
        Some(CoverageItem {
            covered: !self.summary_doc.is_empty(),
            kind: CoverageKind::Summary,
            filename: format!("{base}/docs/summary.md"),
            path: vec![self.name.clone()],
        })
        .into_iter()
        .chain(self.classes.iter().flat_map(|x| x.coverage(&[], files)))
        .chain(self.structs.iter().flat_map(|x| x.coverage(&[], files)))
        .chain(self.enums.iter().flat_map(|x| x.coverage(&[], files)))
        .chain(self.builtins.iter().flat_map(|x| x.coverage(&[], files)))
        .chain(self.constants.iter().flat_map(|x| x.coverage(&[], files)))
    }
}

macro_rules! cov_field {
    ($field: expr, $context: expr, $files: expr) => {{
        let c = $context.clone();
        $field.iter().flat_map(move |x| x.coverage(&c, $files))
    }};
}

impl Class {
    pub fn coverage<'a>(
        &'a self,
        _context: &[String],
        files: &'a Files,
    ) -> impl Iterator<Item = CoverageItem> + 'a {
        let context = self.name.split('.').map(|x| x.to_string()).collect_vec();
        Some(CoverageItem {
            covered: !self.doc_comment.is_empty(),
            kind: CoverageKind::Class,
            filename: files[self.span.get_file()].filename().to_string(),
            path: context.clone(),
        })
        .into_iter()
        .chain(cov_field!(self.public.variables, context, files))
        .chain(cov_field!(self.public.functions, context, files))
        .chain(cov_field!(self.protected.variables, context, files))
        .chain(cov_field!(self.protected.functions, context, files))
        .chain(cov_field!(self.inner_structs, context, files))
        .chain(cov_field!(self.inner_enums, context, files))
        .chain(cov_field!(self.constants, context, files))
        .chain(cov_field!(self.properties, context, files))
        .chain(cov_field!(self.flags, context, files))
    }
}

impl Struct {
    pub fn coverage<'a>(
        &'a self,
        _context: &[String],
        files: &'a Files,
    ) -> impl Iterator<Item = CoverageItem> + 'a {
        let context = self.name.split('.').map(|x| x.to_string()).collect_vec();
        Some(CoverageItem {
            covered: !self.doc_comment.is_empty(),
            kind: CoverageKind::Struct,
            filename: files[self.span.get_file()].filename().to_string(),
            path: context.clone(),
        })
        .into_iter()
        .chain(cov_field!(self.public.variables, context, files))
        .chain(cov_field!(self.public.functions, context, files))
        .chain(cov_field!(self.protected.functions, context, files))
        .chain(cov_field!(self.protected.functions, context, files))
        .chain(cov_field!(self.inner_enums, context, files))
        .chain(cov_field!(self.constants, context, files))
    }
}

impl Enum {
    pub fn coverage<'a>(
        &'a self,
        _context: &[String],
        files: &'a Files,
    ) -> impl Iterator<Item = CoverageItem> + 'a {
        let context = self.name.split('.').map(|x| x.to_string()).collect_vec();
        Some(CoverageItem {
            covered: !self.doc_comment.is_empty(),
            kind: CoverageKind::Enum,
            filename: files[self.span.get_file()].filename().to_string(),
            path: context.clone(),
        })
        .into_iter()
        .chain(cov_field!(self.enumerators, context, files))
    }
}

impl Builtin {
    pub fn coverage<'a>(
        &'a self,
        context: &[String],
        files: &'a Files,
    ) -> impl Iterator<Item = CoverageItem> + 'a {
        let context = context_with(context, &self.name);
        Some(CoverageItem {
            covered: !self.doc_comment.is_empty(),
            kind: CoverageKind::Builtin,
            filename: self.filename.to_string(),
            path: context.clone(),
        })
        .into_iter()
        .chain(cov_field!(self.variables, context, files))
        .chain(cov_field!(self.functions, context, files))
        .chain(cov_field!(self.constants, context, files))
    }
}

impl Constant {
    pub fn coverage(
        &self,
        context: &[String],
        files: &Files,
    ) -> impl Iterator<Item = CoverageItem> {
        let context = context_with(context, &self.name);
        Some(CoverageItem {
            covered: !self.doc_comment.is_empty(),
            kind: CoverageKind::Constant,
            filename: files[self.span.get_file()].filename().to_string(),
            path: context,
        })
        .into_iter()
    }
}

impl MemberVariable {
    pub fn coverage(
        &self,
        context: &[String],
        files: &Files,
    ) -> impl Iterator<Item = CoverageItem> {
        let context = context_with(context, &self.name);
        Some(CoverageItem {
            covered: !self.doc_comment.is_empty(),
            kind: CoverageKind::Member,
            filename: files[self.span.get_file()].filename().to_string(),
            path: context,
        })
        .into_iter()
    }
}

impl Function {
    pub fn coverage(
        &self,
        context: &[String],
        files: &Files,
    ) -> impl Iterator<Item = CoverageItem> {
        let context = context_with(context, &self.name);
        Some(CoverageItem {
            covered: !self.doc_comment.is_empty(),
            kind: CoverageKind::Function,
            filename: files[self.span.get_file()].filename().to_string(),
            path: context,
        })
        .into_iter()
    }
}

impl Enumerator {
    pub fn coverage(
        &self,
        context: &[String],
        files: &Files,
    ) -> impl Iterator<Item = CoverageItem> {
        let context = context_with(context, &self.name);
        Some(CoverageItem {
            covered: !self.doc_comment.is_empty(),
            kind: CoverageKind::Enumerator,
            filename: files[self.span.get_file()].filename().to_string(),
            path: context,
        })
        .into_iter()
    }
}

impl Property {
    pub fn coverage(
        &self,
        context: &[String],
        files: &Files,
    ) -> impl Iterator<Item = CoverageItem> {
        let context = context_with(context, &self.name);
        Some(CoverageItem {
            covered: !self.doc_comment.is_empty(),
            kind: CoverageKind::Property,
            filename: files[self.span.get_file()].filename().to_string(),
            path: context,
        })
        .into_iter()
    }
}

impl Flag {
    pub fn coverage(
        &self,
        context: &[String],
        files: &Files,
    ) -> impl Iterator<Item = CoverageItem> {
        let context = context_with(context, &self.name);
        Some(CoverageItem {
            covered: !self.doc_comment.is_empty(),
            kind: CoverageKind::Flag,
            filename: files[self.span.get_file()].filename().to_string(),
            path: context,
        })
        .into_iter()
    }
}
