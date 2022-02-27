use zscript_parser::Span;

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

#[derive(Debug)]
pub struct MemberVariable {
    pub doc_comment: String,
    pub span: Span,
    pub name: String,
    pub def: SourceCodeWithLinks,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub span: Span,
    pub doc_comment: String,
    pub signature: SourceCodeWithLinks,
}

#[derive(Debug)]
pub struct Constant {
    pub doc_comment: String,
    pub span: Span,
    pub name: String,
    pub def: SourceCodeWithLinks,
}

#[derive(Debug, Default)]
pub struct VariablesAndFunctions {
    pub variables: Vec<MemberVariable>,
    pub functions: Vec<Function>,
}

#[derive(Debug)]
pub struct Class {
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

#[derive(Debug)]
pub struct Struct {
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

#[derive(Debug)]
pub struct Enumerator {
    pub name: String,
    #[allow(unused)]
    pub span: Span,
    pub doc_comment: String,
    pub decl: SourceCodeWithLinks,
}

#[derive(Debug)]
pub struct Enum {
    pub name: String,
    pub no_context_name: String,
    #[allow(unused)]
    pub span: Span,
    pub doc_comment: String,
    pub enumerators: Vec<Enumerator>,
}

#[derive(Debug)]
pub struct Documentation {
    pub name: String,
    pub classes: Vec<Class>,
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub constants: Vec<Constant>,
    pub summary_doc: String,
}
