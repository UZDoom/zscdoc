use zscript_parser::{interner::NameSymbol, Span};

#[derive(Debug, Clone)]
pub enum Owner {
    Class(Vec<String>),
    Struct(Vec<String>),
    Enum(Vec<String>),
    Global,
}

#[derive(Debug, Clone)]
pub enum LinkedSectionKind {
    Struct { link: Vec<String> },
    Class { link: Vec<String> },
    Enum { link: Vec<String> },
    Function { owner: Owner, link: String },
    Member { owner: Owner, link: String },
    Enumerator { owner: Owner, link: String },
    Constant { owner: Owner, link: String },
}

#[derive(Debug)]
pub struct LinkedSection {
    pub text: String,
    pub kind: LinkedSectionKind,
}

#[derive(Debug)]
pub enum SourceCodeSection {
    NoLink(String),
    Linked(LinkedSection),
    PotentialNewlineOnly,
    PotentialNewlineIndent,
    NoNewlineSpacing,
}

#[derive(Debug)]
pub struct SourceCodeWithLinks {
    pub sections: Vec<SourceCodeSection>,
}

pub struct MemberVariable {
    pub context: Vec<NameSymbol>,
    pub doc_comment: String,
    pub span: Span,
    pub name: String,
    pub def: SourceCodeWithLinks,
}

pub struct Function {
    pub context: Vec<NameSymbol>,
    pub name: String,
    pub span: Span,
    pub doc_comment: String,
    pub signature: SourceCodeWithLinks,
}

pub struct Constant {
    pub context: Vec<NameSymbol>,
    pub doc_comment: String,
    pub span: Span,
    pub name: String,
    pub def: SourceCodeWithLinks,
}

#[derive(Default)]
pub struct VariablesAndFunctions {
    pub variables: Vec<MemberVariable>,
    pub functions: Vec<Function>,
}

pub struct Class {
    pub context: Vec<NameSymbol>,
    pub name: String,
    #[allow(unused)]
    pub span: Span,
    pub inherits: Option<String>,
    pub doc_comment: String,
    pub overrides: Vec<Function>,
    pub public: VariablesAndFunctions,
    pub protected: VariablesAndFunctions,
    pub private: VariablesAndFunctions,
    pub inner_structs: Vec<Struct>,
    pub inner_enums: Vec<Enum>,
    pub constants: Vec<Constant>,
}

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
}

pub struct Enumerator {
    pub context: Vec<NameSymbol>,
    pub name: String,
    #[allow(unused)]
    pub span: Span,
    pub doc_comment: String,
    pub decl: SourceCodeWithLinks,
}

pub struct Enum {
    pub context: Vec<NameSymbol>,
    pub name: String,
    pub no_context_name: String,
    #[allow(unused)]
    pub span: Span,
    pub doc_comment: String,
    pub enumerators: Vec<Enumerator>,
}

pub struct Documentation {
    pub name: String,
    pub classes: Vec<Class>,
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub constants: Vec<Constant>,
    pub summary_doc: String,
}
