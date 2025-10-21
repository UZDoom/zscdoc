use zscript_parser::{interner::NameSymbol, Span};

#[derive(Debug, Clone)]
pub enum Owner {
    Class(Vec<String>),
    Struct(Vec<String>),
    Enum(Vec<String>),
    Builtin(String),
    Global,
}

#[derive(Debug, Clone)]
pub enum LinkedSectionKind {
    Struct { link: Vec<String> },
    Class { link: Vec<String> },
    Enum { link: Vec<String> },
    Builtin { link: String },
    Function { owner: Owner, link: String },
    Member { owner: Owner, link: String },
    Enumerator { owner: Owner, link: String },
    Constant { owner: Owner, link: String },
    Property { owner: Owner, link: String },
    Flag { owner: Owner, link: String },
}

#[derive(Debug, Clone)]
pub struct LinkedSection {
    pub link_prefix: Option<String>,
    pub text: String,
    pub kind: LinkedSectionKind,
}

#[derive(Debug, Clone)]
pub enum SourceCodeSection {
    NoLink(String),
    Linked(LinkedSection),
    PotentialNewlineOnly,
    PotentialNewlineIndent,
    NoNewlineSpacing,
}

#[derive(Debug, Clone)]
pub struct SourceCodeWithLinks {
    pub sections: Vec<SourceCodeSection>,
}

#[derive(Debug, Clone)]
pub struct Deprecated {
    pub version: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct MemberVariable {
    pub context: Vec<NameSymbol>,
    pub doc_comment: String,
    pub span: Span,
    pub name: String,
    pub def: SourceCodeWithLinks,
    pub deprecated: Option<Deprecated>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub context: Vec<NameSymbol>,
    pub name: String,
    pub span: Span,
    pub doc_comment: String,
    pub signature: SourceCodeWithLinks,
    pub overrides: Option<LinkedSection>,
    pub deprecated: Option<Deprecated>,
}

#[derive(Debug, Clone)]
pub struct Property {
    pub context: Vec<NameSymbol>,
    pub name: String,
    pub span: Span,
    pub doc_comment: String,
    pub def: SourceCodeWithLinks,
}

#[derive(Debug, Clone)]
pub struct Flag {
    pub context: Vec<NameSymbol>,
    pub name: String,
    pub span: Span,
    pub doc_comment: String,
    pub def: SourceCodeWithLinks,
}

#[derive(Debug, Clone)]
pub struct Constant {
    pub context: Vec<NameSymbol>,
    pub doc_comment: String,
    pub span: Span,
    pub name: String,
    pub def: SourceCodeWithLinks,
}

#[derive(Debug, Clone, Default)]
pub struct VariablesAndFunctions {
    pub variables: Vec<MemberVariable>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct Class {
    pub context: Vec<NameSymbol>,
    pub name: String,
    #[allow(unused)]
    pub span: Span,
    pub inherits: Option<SourceCodeWithLinks>,
    pub doc_comment: String,
    pub overrides: Vec<Function>,
    pub public: VariablesAndFunctions,
    pub protected: VariablesAndFunctions,
    pub private: VariablesAndFunctions,
    pub inner_structs: Vec<Struct>,
    pub inner_enums: Vec<Enum>,
    pub constants: Vec<Constant>,
    pub properties: Vec<Property>,
    pub def_flags: SourceCodeWithLinks,
    pub sealed: Option<SourceCodeWithLinks>,
    pub flags: Vec<Flag>,
    pub deprecated: Option<Deprecated>,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub context: Vec<NameSymbol>,
    pub name: String,
    pub no_context_name: String,
    #[allow(unused)]
    pub span: Span,
    pub doc_comment: String,
    pub public: VariablesAndFunctions,
    pub protected: VariablesAndFunctions,
    pub private: VariablesAndFunctions,
    pub inner_enums: Vec<Enum>,
    pub constants: Vec<Constant>,
    pub def_flags: SourceCodeWithLinks,
    pub deprecated: Option<Deprecated>,
}

#[derive(Debug, Clone)]
pub struct Enumerator {
    pub context: Vec<NameSymbol>,
    pub name: String,
    #[allow(unused)]
    pub span: Span,
    pub doc_comment: String,
    pub decl: SourceCodeWithLinks,
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub context: Vec<NameSymbol>,
    pub name: String,
    pub no_context_name: String,
    #[allow(unused)]
    pub span: Span,
    pub doc_comment: String,
    pub enumerators: Vec<Enumerator>,
}

#[derive(Debug, Clone)]
pub struct Builtin {
    pub context: Vec<NameSymbol>,
    pub name: String,
    pub filename: String,
    pub doc_comment: String,
    pub variables: Vec<MemberVariable>,
    pub functions: Vec<Function>,
    pub constants: Vec<Constant>,
}

#[derive(Debug, Clone)]
pub struct Globals {
    pub variables: Vec<MemberVariable>,
}

pub struct Documentation {
    pub name: String,
    pub classes: Vec<Class>,
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub builtins: Vec<Builtin>,
    pub constants: Vec<Constant>,
    pub globals: Option<Globals>,
    pub summary_doc: String,
}

pub struct Dependency {
    pub link: String,
}

pub struct Dependencies {
    pub dependency_links: Vec<Dependency>,
}
impl Dependencies {
    pub fn get_final_archive_num(&self) -> usize {
        self.dependency_links.len()
    }

    pub fn get_link_prefix(&self, archive_num: usize) -> Option<String> {
        self.dependency_links
            .get(archive_num)
            .map(|x| x.link.clone())
    }
}
