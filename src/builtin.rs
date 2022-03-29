use itertools::Itertools;
use zscript_parser::{
    ast,
    err::{ParsingError, ParsingErrorLevel},
    filesystem::{File, FileIndex, Files},
    hir::{self, lower::HirLowerer},
    interner::NameSymbol,
    ir_common,
    parser::Parser,
};

use crate::{item::ItemProvider, option_slice_to_slice, structures::Owner};

#[derive(serde::Deserialize, Debug)]
pub struct MemberVariableFromFile {
    def: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct FunctionFromFile {
    def: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct ConstantFromFile {
    def: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct BuiltinTypeFromFile {
    #[serde(skip)]
    pub(crate) filename: String,
    name: String,
    doc: String,
    uses_things_from: Option<String>,
    #[serde(alias = "member")]
    members: Option<Vec<MemberVariableFromFile>>,
    #[serde(alias = "function")]
    functions: Option<Vec<FunctionFromFile>>,
    #[serde(alias = "constant")]
    constants: Option<Vec<ConstantFromFile>>,
}

#[derive(Debug)]
pub struct MemberVariableHir {
    pub def: hir::MemberDeclaration,
}

#[derive(Debug)]
pub struct FunctionHir {
    pub def: hir::FunctionDeclaration,
}

#[derive(Debug)]
pub struct ConstantHir {
    pub def: ir_common::ConstDefinition,
}

#[derive(Debug)]
pub struct BuiltinTypeHir {
    pub filename: String,
    pub name: String,
    pub doc: String,
    pub uses_things_from: Option<String>,
    pub members: Vec<MemberVariableHir>,
    pub functions: Vec<FunctionHir>,
    pub constants: Vec<ConstantHir>,
}

impl BuiltinTypeFromFile {
    pub fn produce(self, files: &mut Files) -> Result<BuiltinTypeHir, Vec<ParsingError>> {
        Ok(BuiltinTypeHir {
            doc: self.doc,
            filename: self.filename.clone(),
            name: self.name,
            uses_things_from: self.uses_things_from,
            members: option_slice_to_slice(self.members.as_deref())
                .iter()
                .map(|m| m.produce(files, &self.filename))
                .collect::<Result<Vec<_>, _>>()?,
            functions: option_slice_to_slice(self.functions.as_deref())
                .iter()
                .map(|m| m.produce(files, &self.filename))
                .collect::<Result<Vec<_>, _>>()?,
            constants: option_slice_to_slice(self.constants.as_deref())
                .iter()
                .map(|m| m.produce(files, &self.filename))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

fn parse_inner<T>(
    file_index: FileIndex,
    def: &str,
) -> Result<
    (
        ast::ClassInner,
        Vec<ParsingError>,
        impl Fn(&str) -> Result<T, Vec<ParsingError>>,
    ),
    Vec<ParsingError>,
> {
    let mut parser = Parser::new(file_index, def);
    let parsed = parser.get_class_inner().map_err(|x| vec![x])?;
    let parsed = parser
        .expect(parsed, "a class inner")
        .map_err(|x| vec![x])?;
    let parsed_span = parsed.span;
    let err = move |msg: &str| {
        Err(vec![ParsingError {
            level: ParsingErrorLevel::Error,
            msg: msg.to_string(),
            main_spans: vec1::vec1![parsed_span],
            info_spans: vec![],
        }])
    };
    let errs = parser.to_errs();
    if !errs.is_empty() {
        return Err(errs);
    }
    Ok((parsed, errs, err))
}

impl MemberVariableFromFile {
    fn produce(
        &self,
        files: &mut Files,
        filename: &str,
    ) -> Result<MemberVariableHir, Vec<ParsingError>> {
        let file = File::new(
            filename.to_string() + " member variable",
            self.def.as_bytes().to_vec(),
        );
        let file_index = files.add(file);
        let (parsed, mut errs, err) = parse_inner(file_index, &self.def)?;
        let member = match parsed.kind {
            ast::ClassInnerKind::Declaration(ast::Declaration::Member(m)) => m,
            _ => {
                return err("needed member variable");
            }
        };
        let mut lowerer = HirLowerer::new(&mut errs);
        let mut members = lowerer
            .lower_member_declaration(member, false)
            .collect_vec();
        if !errs.is_empty() {
            return Err(errs);
        }
        let m = if members.len() == 1 {
            members.pop().unwrap()
        } else {
            return err("should get only one member variable");
        };
        Ok(MemberVariableHir { def: m })
    }
}

impl FunctionFromFile {
    fn produce(&self, files: &mut Files, filename: &str) -> Result<FunctionHir, Vec<ParsingError>> {
        let file = File::new(
            filename.to_string() + " function",
            self.def.as_bytes().to_vec(),
        );
        let file_index = files.add(file);
        let (parsed, mut errs, err) = parse_inner(file_index, &self.def)?;
        let function = match parsed.kind {
            ast::ClassInnerKind::Declaration(ast::Declaration::Function(f)) => f,
            _ => {
                return err("needed function");
            }
        };
        let mut lowerer = HirLowerer::new(&mut errs);
        let f = lowerer.lower_function_declaration(function);
        if !errs.is_empty() {
            return Err(errs);
        }
        Ok(FunctionHir { def: f })
    }
}

impl ConstantFromFile {
    fn produce(&self, files: &mut Files, filename: &str) -> Result<ConstantHir, Vec<ParsingError>> {
        let file = File::new(
            filename.to_string() + " constant",
            self.def.as_bytes().to_vec(),
        );
        let file_index = files.add(file);
        let (parsed, errs, err) = parse_inner(file_index, &self.def)?;
        if !errs.is_empty() {
            return Err(errs);
        }
        let c = match parsed.kind {
            ast::ClassInnerKind::Const(c) => c,
            _ => {
                return err("needed constant");
            }
        };
        Ok(ConstantHir { def: c })
    }
}

impl BuiltinTypeHir {
    pub fn produce(
        &self,
        files: &mut Files,
        item_provider: &ItemProvider,
    ) -> crate::structures::Builtin {
        let context = vec![zscript_parser::interner::intern_name(&self.name)];
        crate::structures::Builtin {
            context: context.clone(),
            name: self.name.clone(),
            filename: self.filename.clone(),
            doc_comment: self.doc.to_string(),
            variables: self
                .members
                .iter()
                .map(|m| m.produce(files, item_provider, context.clone()))
                .collect_vec(),
            functions: self
                .functions
                .iter()
                .map(|m| m.produce(files, item_provider, context.clone()))
                .collect_vec(),
            constants: self
                .constants
                .iter()
                .map(|m| m.produce(files, item_provider, context.clone()))
                .collect_vec(),
        }
    }

    pub fn extend_with_uses_things_from(&mut self, hir: &hir::TopLevel) -> anyhow::Result<()> {
        use anyhow::Context;
        let mut funcs_to_add = vec![];
        let mut vars_to_add = vec![];
        let mut consts_to_add = vec![];
        if let Some(n) = &self.uses_things_from {
            let ns = zscript_parser::interner::intern_name(n);
            let h = hir
                .definitions
                .get(&ns)
                .and_then(|x| {
                    x.iter().find_map(|i| match &i.kind {
                        zscript_parser::hir::TopLevelDefinitionKind::Struct(s) => Some(s),
                        _ => None,
                    })
                })
                .context(format!(
                    "expected to get a top-level struct element {n} for a builtin"
                ))?;
            for i in h.inners.values() {
                let i = &i[0];
                match &i.kind {
                    zscript_parser::hir::StructInnerKind::FunctionDeclaration(f) => {
                        funcs_to_add.push(FunctionHir { def: f.clone() });
                    }
                    zscript_parser::hir::StructInnerKind::MemberDeclaration(m) => {
                        vars_to_add.push(MemberVariableHir { def: m.clone() });
                    }
                    zscript_parser::hir::StructInnerKind::Const(c) => {
                        consts_to_add.push(ConstantHir { def: c.clone() });
                    }
                    _ => {}
                }
            }
        }
        funcs_to_add.sort_unstable_by_key(|x| x.def.span);
        vars_to_add.sort_unstable_by_key(|x| x.def.span);
        consts_to_add.sort_unstable_by_key(|x| x.def.span);
        self.functions.extend(funcs_to_add);
        self.members.extend(vars_to_add);
        Ok(())
    }
}

impl MemberVariableHir {
    fn produce(
        &self,
        files: &mut Files,
        item_provider: &ItemProvider,
        context: Vec<NameSymbol>,
    ) -> crate::structures::MemberVariable {
        let var_to_add = crate::structures::MemberVariable {
            context,
            name: files.text_from_span(self.def.name.span).to_string(),
            span: self.def.span,
            doc_comment: self
                .def
                .doc_comment
                .map(|s| s.string().to_string())
                .unwrap_or_else(|| "".to_string()),
            def: crate::document::reconstruct_member_declaration(
                Owner::Global,
                &self.def,
                item_provider,
                &[],
                files,
            ),
            deprecated: self
                .def
                .deprecated
                .as_ref()
                .map(crate::document::transform_deprecated),
        };
        var_to_add
    }
}

impl FunctionHir {
    fn produce(
        &self,
        files: &mut Files,
        item_provider: &ItemProvider,
        context: Vec<NameSymbol>,
    ) -> crate::structures::Function {
        let func_to_add = crate::structures::Function {
            context,
            name: files.text_from_span(self.def.name.span).to_string(),
            span: self.def.span,
            doc_comment: self
                .def
                .doc_comment
                .map(|s| s.string().to_string())
                .unwrap_or_else(|| "".to_string()),
            signature: crate::document::reconstruct_function_signature(
                Owner::Global,
                &self.def,
                item_provider,
                &[],
                files,
            ),
            overrides: None,
            deprecated: self
                .def
                .deprecated
                .as_ref()
                .map(crate::document::transform_deprecated),
        };
        func_to_add
    }
}

impl ConstantHir {
    fn produce(
        &self,
        files: &mut Files,
        item_provider: &ItemProvider,
        context: Vec<NameSymbol>,
    ) -> crate::structures::Constant {
        let const_to_add = crate::structures::Constant {
            context,
            name: files.text_from_span(self.def.name.span).to_string(),
            span: self.def.span,
            doc_comment: self
                .def
                .doc_comment
                .map(|s| s.string().to_string())
                .unwrap_or_else(|| "".to_string()),
            def: crate::document::reconstruct_constant_declaration(
                Owner::Global,
                &self.def,
                item_provider,
                files,
            ),
        };
        const_to_add
    }
}
