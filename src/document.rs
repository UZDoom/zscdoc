use crate::structures::*;
use zscript_parser::{
    filesystem::Files,
    hir::{self, ItemProvider},
    ir_common::{self, Identifier},
};

impl SourceCodeWithLinks {
    fn add_no_link(&mut self, text: &str) {
        if let Some(SourceCodeSection::NoLink(s)) = self.sections.last_mut() {
            *s += text;
        } else {
            self.sections
                .push(SourceCodeSection::NoLink(text.to_string()));
        }
    }

    fn add_link(&mut self, text: &str, kind: LinkedSectionKind) {
        self.sections.push(SourceCodeSection::Linked(LinkedSection {
            text: text.to_string(),
            kind,
        }));
    }

    fn add_newline_indent_or_spacing(&mut self) {
        self.add_newline_indent();
        self.sections.push(SourceCodeSection::NoNewlineSpacing);
    }

    fn add_newline_indent(&mut self) {
        self.sections
            .push(SourceCodeSection::PotentialNewlineIndent);
    }

    fn add_newline_no_indent(&mut self) {
        self.sections.push(SourceCodeSection::PotentialNewlineOnly);
    }
}

const FUNCTION_FLAG_ORDER: [hir::FunctionFlags; 14] = [
    hir::FunctionFlags::PRIVATE,
    hir::FunctionFlags::PROTECTED,
    hir::FunctionFlags::NATIVE,
    hir::FunctionFlags::STATIC,
    hir::FunctionFlags::VIRTUAL,
    hir::FunctionFlags::ABSTRACT,
    hir::FunctionFlags::OVERRIDE,
    hir::FunctionFlags::FINAL,
    hir::FunctionFlags::VAR_ARG,
    hir::FunctionFlags::UI,
    hir::FunctionFlags::PLAY,
    hir::FunctionFlags::CLEAR_SCOPE,
    hir::FunctionFlags::VIRTUAL_SCOPE,
    hir::FunctionFlags::TRANSIENT,
];

fn function_flag_to_string(flag: hir::FunctionFlags) -> &'static str {
    match flag {
        hir::FunctionFlags::NATIVE => "native",
        hir::FunctionFlags::STATIC => "static",
        hir::FunctionFlags::PRIVATE => "private",
        hir::FunctionFlags::PROTECTED => "protected",
        hir::FunctionFlags::FINAL => "final",
        hir::FunctionFlags::TRANSIENT => "transient",
        hir::FunctionFlags::VIRTUAL => "virtual",
        hir::FunctionFlags::OVERRIDE => "override",
        hir::FunctionFlags::ABSTRACT => "abstract",
        hir::FunctionFlags::VAR_ARG => "vararg",
        hir::FunctionFlags::UI => "ui",
        hir::FunctionFlags::PLAY => "play",
        hir::FunctionFlags::CLEAR_SCOPE => "clearscope",
        hir::FunctionFlags::VIRTUAL_SCOPE => "virtualscope",
        _ => panic!(),
    }
}

fn resolve_type_if_possible(
    fallback: &str,
    chain: &[Identifier],
    hir: &hir::TopLevel,
    files: &Files,
    prefix_dot_if_long_chain: bool,
    source: &mut SourceCodeWithLinks,
) {
    let mut chain = chain.iter();
    let mut cur_provider = Some(hir as &dyn ItemProvider);
    let mut cur_link = vec![];
    let mut cur_link_sections = vec![];
    let success = loop {
        let id = chain.next();
        let id = if let Some(id) = id {
            id
        } else {
            break true;
        };
        let provider = if let Some(p) = cur_provider {
            p
        } else {
            break false;
        };
        let thing = if let Some(p) = provider.get_one(id.symbol) {
            p
        } else {
            break false;
        };
        cur_provider = thing.item_provider();
        macro_rules! add {
            ($type: ident, $name: expr) => {{
                cur_link.push($name.clone());
                cur_link_sections.push((
                    $name.clone(),
                    LinkedSectionKind::$type {
                        link: cur_link.clone(),
                    },
                ));
            }};
        }
        match thing {
            hir::Item::Class(c) => {
                let name = files.text_from_span(c.name.span).to_string();
                add!(Class, name)
            }
            hir::Item::Struct(s) => {
                let name = files.text_from_span(s.name.span).to_string();
                add!(Struct, name)
            }
            hir::Item::Enum(e) => {
                let name = files.text_from_span(e.name.span).to_string();
                add!(Enum, name)
            }
            hir::Item::Const(_c) => {
                // TODO
                break false;
            }
            hir::Item::Variant {
                enum_ref: _,
                variant_name: _,
            } => {
                // TODO
                break false;
            }
            hir::Item::StaticConstArray(_sca) => {
                // TODO
                break false;
            }
            hir::Item::FunctionDeclaration(_f) => {
                // TODO
                break false;
            }
            hir::Item::MemberDeclaration(_m) => {
                // TODO
                break false;
            }
        };
    };
    if success {
        if prefix_dot_if_long_chain && cur_link_sections.len() > 1 {
            source.add_no_link(".");
        }
        let mut first = true;
        for (text, sec) in cur_link_sections {
            if !first {
                source.add_no_link(".");
            }
            first = false;
            source.add_link(&text, sec);
        }
    } else {
        source.add_no_link(fallback);
    }
}

fn add_type_to_source(
    ty: &hir::Type,
    hir: &hir::TopLevel,
    source: &mut SourceCodeWithLinks,
    files: &Files,
) {
    match ty {
        hir::Type::SingleUserType(id) => {
            resolve_type_if_possible(
                files.text_from_span(id.span),
                &[*id],
                hir,
                files,
                true,
                source,
            );
        }
        hir::Type::DottedUserType(ids) => {
            resolve_type_if_possible(
                files.text_from_span(ids.span),
                &ids.ids,
                hir,
                files,
                true,
                source,
            );
        }
        hir::Type::NativeType(id) => {
            source.add_no_link("@");
            resolve_type_if_possible(
                files.text_from_span(id.span),
                &[*id],
                hir,
                files,
                true,
                source,
            );
        }
        hir::Type::ReadonlyType(id) => {
            source.add_no_link("ReadOnly< ");
            resolve_type_if_possible(
                files.text_from_span(id.span),
                &[*id],
                hir,
                files,
                true,
                source,
            );
            source.add_no_link(" >");
        }
        hir::Type::ReadonlyNativeType(id) => {
            source.add_no_link("ReadOnly< @");
            resolve_type_if_possible(
                files.text_from_span(id.span),
                &[*id],
                hir,
                files,
                true,
                source,
            );
            source.add_no_link(" >");
        }
        hir::Type::Class(ids) => {
            source.add_no_link("Class< ");
            match ids {
                Some(ids) => {
                    resolve_type_if_possible(
                        files.text_from_span(ids.span),
                        &ids.ids,
                        hir,
                        files,
                        false,
                        source,
                    );
                }
                None => {
                    source.add_no_link("Object");
                }
            }
            source.add_no_link(" >");
        }
        hir::Type::Map(b) => {
            let (k, v) = &**b;
            source.add_no_link("Map< ");
            add_type_to_source(k, hir, source, files);
            source.add_no_link(", ");
            add_type_to_source(v, hir, source, files);
            source.add_no_link(" >");
        }
        hir::Type::Array(initial_cty, initial_size) => {
            let mut sizes = vec![];
            // this is a mess because while creating HIR the parser makes arrays into a more useful
            // recursive definition
            // unfortunately that makes extracting back out the annoying C syntax messy
            let mut cty = &**initial_cty;
            let mut size = &*initial_size;
            loop {
                sizes.push(size);
                if let hir::Type::Array(new_cty, new_size) = cty {
                    cty = new_cty;
                    size = new_size;
                } else {
                    add_type_to_source(cty, hir, source, files);
                    for s in sizes {
                        source.add_no_link(&format!(
                            "[{}]",
                            s.as_ref()
                                .map(|s| files.text_from_span(s.span.unwrap()))
                                .unwrap_or(""),
                        ));
                    }
                    break;
                }
            }
        }
        hir::Type::DynArray(d) => {
            source.add_no_link("Array< ");
            add_type_to_source(d, hir, source, files);
            source.add_no_link(" >");
        }
        hir::Type::Let => {
            source.add_no_link("let");
        }
        hir::Type::Error => {
            source.add_no_link("{unknown}");
        }
    }
}

fn reconstruct_function_signature(
    owner: Owner,
    func: &hir::FunctionDeclaration,
    hir: &hir::TopLevel,
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    for f in FUNCTION_FLAG_ORDER {
        if func.flags.contains(f) {
            ret.add_no_link(function_flag_to_string(f));
            ret.add_no_link(" ");
        }
    }
    match &func.return_types.kind {
        hir::TypeListOrVoidKind::TypeList(l) => {
            let mut first = true;
            for ty in l {
                if !first {
                    ret.add_no_link(", ");
                }
                first = false;
                add_type_to_source(ty, hir, &mut ret, files);
            }
            ret.add_no_link(" ");
        }
        hir::TypeListOrVoidKind::Void => {
            ret.add_no_link("void ");
        }
    }

    let name = files.text_from_span(func.name.span).to_string();
    ret.add_link(
        &name,
        LinkedSectionKind::Function {
            owner,
            link: name.clone(),
        },
    );

    ret.add_no_link("(");
    ret.add_newline_indent();
    let mut first = true;
    for p in func.params.args.iter() {
        if !first {
            ret.add_no_link(",");
            ret.add_newline_indent_or_spacing();
        }
        first = false;
        for f in [
            hir::FuncParamFlags::IN,
            hir::FuncParamFlags::OUT,
            hir::FuncParamFlags::OPTIONAL,
        ] {
            if p.flags.contains(f) {
                ret.add_no_link(match f {
                    hir::FuncParamFlags::IN => "in",
                    hir::FuncParamFlags::OUT => "out",
                    hir::FuncParamFlags::OPTIONAL => "optional",
                    _ => unreachable!(),
                });
                ret.add_no_link(" ");
            }
        }
        add_type_to_source(&p.param_type, hir, &mut ret, files);
        ret.add_no_link(" ");
        ret.add_no_link(files.text_from_span(p.name.span));
        if let Some(e) = &p.init {
            ret.add_no_link(" = ");
            ret.add_no_link(files.text_from_span(e.span.unwrap()));
        }
    }
    if func.params.variadic {
        if !first {
            ret.add_no_link(",");
            ret.add_newline_indent_or_spacing();
        }
        ret.add_no_link("...");
    }
    ret.add_newline_no_indent();
    ret.add_no_link(")");
    if func.constant {
        ret.add_no_link(" const");
    }
    ret
}

const MEMBER_FLAG_ORDER: [hir::MemberFlags; 11] = [
    hir::MemberFlags::NATIVE,
    hir::MemberFlags::PRIVATE,
    hir::MemberFlags::PROTECTED,
    hir::MemberFlags::TRANSIENT,
    hir::MemberFlags::READ_ONLY,
    hir::MemberFlags::INTERNAL,
    hir::MemberFlags::VAR_ARG,
    hir::MemberFlags::UI,
    hir::MemberFlags::PLAY,
    hir::MemberFlags::CLEAR_SCOPE,
    hir::MemberFlags::META,
];

fn member_flag_to_string(flag: hir::MemberFlags) -> &'static str {
    match flag {
        hir::MemberFlags::NATIVE => "native",
        hir::MemberFlags::PRIVATE => "private",
        hir::MemberFlags::PROTECTED => "protected",
        hir::MemberFlags::TRANSIENT => "transient",
        hir::MemberFlags::READ_ONLY => "readonly",
        hir::MemberFlags::INTERNAL => "internal",
        hir::MemberFlags::VAR_ARG => "vararg",
        hir::MemberFlags::UI => "ui",
        hir::MemberFlags::PLAY => "play",
        hir::MemberFlags::CLEAR_SCOPE => "clearscope",
        hir::MemberFlags::META => "meta",
        _ => panic!(),
    }
}

fn reconstruct_member_declaration(
    owner: Owner,
    member: &hir::MemberDeclaration,
    hir: &hir::TopLevel,
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    for f in MEMBER_FLAG_ORDER {
        if member.flags.contains(f) {
            ret.add_no_link(member_flag_to_string(f));
            ret.add_no_link(" ");
        }
    }
    add_type_to_source(&member.member_type, hir, &mut ret, files);
    ret.add_no_link(" ");
    let name = files.text_from_span(member.name.span).to_string();
    ret.add_link(
        &name,
        LinkedSectionKind::Member {
            owner,
            link: name.clone(),
        },
    );
    ret
}

fn reconstruct_enumerator_declaration(
    owner: Owner,
    variant: &ir_common::EnumVariant,
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    let name = files.text_from_span(variant.name.span).to_string();
    ret.add_link(
        &name,
        LinkedSectionKind::Enumerator {
            owner,
            link: name.clone(),
        },
    );
    if let Some(e) = &variant.init {
        ret.add_no_link(" = ");
        ret.add_no_link(files.text_from_span(e.span.unwrap()));
    }
    ret
}

fn reconstruct_constant_declaration(
    owner: Owner,
    constant: &ir_common::ConstDefinition,
    _hir: &hir::TopLevel,
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    ret.add_no_link("const ");
    let name = files.text_from_span(constant.name.span).to_string();
    ret.add_link(
        &name,
        LinkedSectionKind::Constant {
            owner,
            link: name.clone(),
        },
    );
    ret.add_no_link(" = ");
    ret.add_no_link(files.text_from_span(constant.expr.span.unwrap()));
    ret
}

fn reconstruct_static_const_array_declaration(
    owner: Owner,
    sca: &hir::StaticConstArray,
    hir: &hir::TopLevel,
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    ret.add_no_link("static const ");
    add_type_to_source(&sca.arr_type, hir, &mut ret, files);
    ret.add_no_link("[] ");
    let name = files.text_from_span(sca.name.span).to_string();
    ret.add_link(
        &name,
        LinkedSectionKind::Constant {
            owner,
            link: name.clone(),
        },
    );
    ret.add_no_link(" = {");
    ret.add_newline_indent();
    let mut first = true;
    for e in sca.exprs.list.iter() {
        if !first {
            ret.add_no_link(",");
            ret.add_newline_indent_or_spacing();
        }
        first = false;
        ret.add_no_link(files.text_from_span(e.span.unwrap()));
    }
    ret.add_newline_no_indent();
    ret.add_no_link("}");
    ret
}

fn class_doc(name: &str, hir: &hir::TopLevel, c: &hir::ClassDefinition, files: &Files) -> Class {
    let mut class_to_add = Class {
        name: name.to_string(),
        span: c.span,
        inherits: c.ancestor.map(|a| files.text_from_span(a.span).to_string()),
        doc_comment: c
            .doc_comment
            .map(|s| s.string().to_string())
            .unwrap_or_else(|| "".to_string()),
        overrides: vec![],
        public: VariablesAndFunctions::default(),
        protected: VariablesAndFunctions::default(),
        private: VariablesAndFunctions::default(),
        inner_structs: vec![],
        inner_enums: vec![],
        constants: vec![],
    };
    for (_, node) in c.inners.iter() {
        let inner_name = files.text_from_span(node[0].name().span);
        match &node[0].kind {
            hir::ClassInnerKind::FunctionDeclaration(f) => {
                let owner = Owner::Class(vec![name.to_string()]);
                let func_to_add = Function {
                    name: inner_name.to_string(),
                    span: f.span,
                    doc_comment: f
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    signature: reconstruct_function_signature(owner, f, hir, files),
                };
                if f.flags.contains(hir::FunctionFlags::OVERRIDE) {
                    class_to_add.overrides.push(func_to_add);
                } else if f.flags.contains(hir::FunctionFlags::PRIVATE) {
                    class_to_add.private.functions.push(func_to_add);
                } else if f.flags.contains(hir::FunctionFlags::PROTECTED) {
                    class_to_add.protected.functions.push(func_to_add);
                } else {
                    class_to_add.public.functions.push(func_to_add);
                }
            }
            hir::ClassInnerKind::MemberDeclaration(m) => {
                let owner = Owner::Class(vec![name.to_string()]);
                let var_to_add = MemberVariable {
                    name: inner_name.to_string(),
                    span: m.span,
                    doc_comment: m
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    def: reconstruct_member_declaration(owner, m, hir, files),
                };
                if m.flags.contains(hir::MemberFlags::PRIVATE) {
                    class_to_add.private.variables.push(var_to_add);
                } else if m.flags.contains(hir::MemberFlags::PROTECTED) {
                    class_to_add.protected.variables.push(var_to_add);
                } else {
                    class_to_add.public.variables.push(var_to_add);
                }
            }
            hir::ClassInnerKind::Struct(s) => {
                let struct_to_add =
                    struct_doc(&format!("{name}.{inner_name}"), inner_name, hir, s, files);
                class_to_add.inner_structs.push(struct_to_add);
            }
            hir::ClassInnerKind::Enum(e) => {
                let enum_to_add =
                    enum_doc(&format!("{name}.{inner_name}"), inner_name, hir, e, files);
                class_to_add.inner_enums.push(enum_to_add);
            }
            hir::ClassInnerKind::Const(c) => {
                let owner = Owner::Class(vec![name.to_string()]);
                let const_to_add = Constant {
                    name: inner_name.to_string(),
                    doc_comment: c
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: c.span,
                    def: reconstruct_constant_declaration(owner, c, hir, files),
                };
                class_to_add.constants.push(const_to_add);
            }
            hir::ClassInnerKind::StaticConstArray(sca) => {
                let owner = Owner::Class(vec![name.to_string()]);
                let const_to_add = Constant {
                    name: inner_name.to_string(),
                    doc_comment: sca
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: sca.span,
                    def: reconstruct_static_const_array_declaration(owner, sca, hir, files),
                };
                class_to_add.constants.push(const_to_add);
            }
            _ => {}
        }
    }
    class_to_add.constants.sort_unstable_by_key(|x| x.span);
    for mf in [
        &mut class_to_add.public,
        &mut class_to_add.protected,
        &mut class_to_add.private,
    ] {
        mf.functions.sort_unstable_by_key(|x| x.span);
        mf.variables.sort_unstable_by_key(|x| x.span);
    }
    class_to_add
}

fn struct_doc(
    name: &str,
    no_context_name: &str,
    hir: &hir::TopLevel,
    s: &hir::StructDefinition,
    files: &Files,
) -> Struct {
    let mut struct_to_add = Struct {
        name: name.to_string(),
        no_context_name: no_context_name.to_string(),
        span: s.span,
        doc_comment: s
            .doc_comment
            .map(|s| s.string().to_string())
            .unwrap_or_else(|| "".to_string()),
        public: VariablesAndFunctions::default(),
        protected: VariablesAndFunctions::default(),
        private: VariablesAndFunctions::default(),
        inner_enums: vec![],
        constants: vec![],
    };
    for (_, node) in s.inners.iter() {
        let inner_name = files.text_from_span(node[0].name().span);
        match &node[0].kind {
            hir::StructInnerKind::FunctionDeclaration(f) => {
                let owner = Owner::Struct(vec![name.to_string()]);
                let func_to_add = Function {
                    name: inner_name.to_string(),
                    span: f.span,
                    doc_comment: f
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    signature: reconstruct_function_signature(owner, f, hir, files),
                };
                if f.flags.contains(hir::FunctionFlags::PRIVATE) {
                    struct_to_add.private.functions.push(func_to_add);
                } else if f.flags.contains(hir::FunctionFlags::PROTECTED) {
                    struct_to_add.protected.functions.push(func_to_add);
                } else {
                    struct_to_add.public.functions.push(func_to_add);
                }
            }
            hir::StructInnerKind::MemberDeclaration(m) => {
                let owner = Owner::Struct(vec![name.to_string()]);
                let var_to_add = MemberVariable {
                    name: inner_name.to_string(),
                    span: m.span,
                    doc_comment: m
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    def: reconstruct_member_declaration(owner, m, hir, files),
                };
                if m.flags.contains(hir::MemberFlags::PRIVATE) {
                    struct_to_add.private.variables.push(var_to_add);
                } else if m.flags.contains(hir::MemberFlags::PROTECTED) {
                    struct_to_add.protected.variables.push(var_to_add);
                } else {
                    struct_to_add.public.variables.push(var_to_add);
                }
            }
            hir::StructInnerKind::Enum(e) => {
                let enum_to_add =
                    enum_doc(&format!("{name}.{inner_name}"), inner_name, hir, e, files);
                struct_to_add.inner_enums.push(enum_to_add);
            }
            hir::StructInnerKind::Const(c) => {
                let owner = Owner::Struct(vec![name.to_string()]);
                let const_to_add = Constant {
                    name: inner_name.to_string(),
                    doc_comment: c
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: c.span,
                    def: reconstruct_constant_declaration(owner, c, hir, files),
                };
                struct_to_add.constants.push(const_to_add);
            }
            hir::StructInnerKind::StaticConstArray(sca) => {
                let owner = Owner::Struct(vec![name.to_string()]);
                let const_to_add = Constant {
                    name: inner_name.to_string(),
                    doc_comment: sca
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: sca.span,
                    def: reconstruct_static_const_array_declaration(owner, sca, hir, files),
                };
                struct_to_add.constants.push(const_to_add);
            }
        }
    }
    struct_to_add.constants.sort_unstable_by_key(|x| x.span);
    for mf in [
        &mut struct_to_add.public,
        &mut struct_to_add.protected,
        &mut struct_to_add.private,
    ] {
        mf.functions.sort_unstable_by_key(|x| x.span);
        mf.variables.sort_unstable_by_key(|x| x.span);
    }
    struct_to_add
}

fn enum_doc(
    name: &str,
    no_context_name: &str,
    _hir: &hir::TopLevel,
    e: &ir_common::EnumDefinition,
    files: &Files,
) -> Enum {
    let mut enum_to_add = Enum {
        name: name.to_string(),
        no_context_name: no_context_name.to_string(),
        span: e.span,
        doc_comment: e
            .doc_comment
            .map(|s| s.string().to_string())
            .unwrap_or_else(|| "".to_string()),
        enumerators: vec![],
    };
    for i in e.variants.iter() {
        let inner_name = files.text_from_span(i.name.span);
        let enumerator_to_add = Enumerator {
            name: inner_name.to_string(),
            span: i.span,
            doc_comment: i
                .doc_comment
                .map(|s| s.string().to_string())
                .unwrap_or_else(|| "".to_string()),
            decl: reconstruct_enumerator_declaration(Owner::Enum(vec![name.to_string()]), i, files),
        };
        enum_to_add.enumerators.push(enumerator_to_add);
    }
    enum_to_add
}

pub fn hir_to_doc_structures(
    nice_name: String,
    hir: &hir::TopLevel,
    files: &Files,
) -> Documentation {
    let mut docs = Documentation {
        name: nice_name,
        classes: vec![],
        structs: vec![],
        enums: vec![],
        constants: vec![],
    };
    for (_, node) in hir.definitions.iter() {
        let name = files.text_from_span(node[0].name().span);
        match &node[0].kind {
            hir::TopLevelDefinitionKind::Class(c) => {
                let class_to_add = class_doc(name, hir, c, files);
                docs.classes.push(class_to_add);
            }
            hir::TopLevelDefinitionKind::Struct(s) => {
                let struct_to_add = struct_doc(name, name, hir, s, files);
                docs.structs.push(struct_to_add);
            }
            hir::TopLevelDefinitionKind::Enum(e) => {
                let enum_to_add = enum_doc(name, name, hir, e, files);
                docs.enums.push(enum_to_add);
            }
            hir::TopLevelDefinitionKind::Const(c) => {
                let owner = Owner::Global;
                let const_to_add = Constant {
                    name: name.to_string(),
                    doc_comment: c
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: c.span,
                    def: reconstruct_constant_declaration(owner, c, hir, files),
                };
                docs.constants.push(const_to_add);
            }
            hir::TopLevelDefinitionKind::MixinClass(_m) => { /* TODO */ }
        }
    }
    docs.classes.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    docs.structs.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    docs.enums.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    docs.constants.sort_unstable_by_key(|x| x.span);
    docs
}
