use crate::{
    item::{context_with, ItemProvider},
    structures::*,
};
use zscript_parser::{
    filesystem::Files,
    hir,
    interner::{NameSymbol, StringSymbol},
    ir_common,
};

fn should_skip(doc_comment: Option<&StringSymbol>) -> bool {
    let doc_comment = if let Some(d) = doc_comment {
        d
    } else {
        return false;
    };
    let doc_comment = doc_comment.string();
    doc_comment.trim_start().starts_with("?doc: hidden")
}

impl SourceCodeWithLinks {
    fn add_no_link(&mut self, text: &str) {
        if let Some(SourceCodeSection::NoLink(s)) = self.sections.last_mut() {
            *s += text;
        } else {
            self.sections
                .push(SourceCodeSection::NoLink(text.to_string()));
        }
    }

    fn add_link(&mut self, sec: LinkedSection) {
        self.sections.push(SourceCodeSection::Linked(sec));
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

fn add_type_if_possible<T, U>(
    fallback: &str,
    chain: U,
    item_provider: &ItemProvider,
    context: &[NameSymbol],
    prefix_dot_if_long_chain: bool,
    source: &mut SourceCodeWithLinks,
) where
    T: Into<NameSymbol>,
    U: IntoIterator<Item = T>,
    U::IntoIter: Clone,
{
    match item_provider.resolve(context, chain.into_iter().map(|x| x.into())) {
        Some(cur_link_sections) => {
            if prefix_dot_if_long_chain && cur_link_sections.len() > 1 {
                source.add_no_link(".");
            }
            let mut first = true;
            for sec in cur_link_sections {
                if !first {
                    source.add_no_link(".");
                }
                first = false;
                source.add_link((*sec).clone());
            }
        }
        None => {
            source.add_no_link(fallback);
        }
    }
}

fn add_type_to_source(
    ty: &hir::Type,
    item_provider: &ItemProvider,
    context: &[NameSymbol],
    source: &mut SourceCodeWithLinks,
    files: &Files,
) {
    match ty {
        hir::Type::SingleUserType(id) => {
            add_type_if_possible(
                files.text_from_span(id.span),
                &[*id],
                item_provider,
                context,
                true,
                source,
            );
        }
        hir::Type::DottedUserType(ids) => {
            add_type_if_possible(
                files.text_from_span(ids.span),
                &ids.ids,
                item_provider,
                context,
                true,
                source,
            );
        }
        hir::Type::NativeType(id) => {
            source.add_no_link("@");
            add_type_if_possible(
                files.text_from_span(id.span),
                &[*id],
                item_provider,
                context,
                true,
                source,
            );
        }
        hir::Type::ReadonlyType(id) => {
            source.add_no_link("ReadOnly< ");
            add_type_if_possible(
                files.text_from_span(id.span),
                &[*id],
                item_provider,
                context,
                true,
                source,
            );
            source.add_no_link(" >");
        }
        hir::Type::ReadonlyNativeType(id) => {
            source.add_no_link("ReadOnly< @");
            add_type_if_possible(
                files.text_from_span(id.span),
                &[*id],
                item_provider,
                context,
                true,
                source,
            );
            source.add_no_link(" >");
        }
        hir::Type::Class(ids) => {
            source.add_no_link("Class< ");
            match ids {
                Some(ids) => {
                    add_type_if_possible(
                        files.text_from_span(ids.span),
                        &ids.ids,
                        item_provider,
                        context,
                        false,
                        source,
                    );
                }
                None => {
                    add_type_if_possible(
                        "Object",
                        [zscript_parser::interner::intern_name("Object")],
                        item_provider,
                        context,
                        false,
                        source,
                    );
                }
            }
            source.add_no_link(" >");
        }
        hir::Type::Map(b) => {
            let (k, v) = &**b;
            source.add_no_link("Map< ");
            add_type_to_source(k, item_provider, context, source, files);
            source.add_no_link(", ");
            add_type_to_source(v, item_provider, context, source, files);
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
                    add_type_to_source(cty, item_provider, context, source, files);
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
            add_type_to_source(d, item_provider, context, source, files);
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
    item_provider: &ItemProvider,
    context: &[NameSymbol],
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
                add_type_to_source(ty, item_provider, context, &mut ret, files);
            }
            ret.add_no_link(" ");
        }
        hir::TypeListOrVoidKind::Void => {
            ret.add_no_link("void ");
        }
    }

    let name = files.text_from_span(func.name.span).to_string();
    ret.add_link(LinkedSection {
        link_prefix: None,
        text: name.clone(),
        kind: LinkedSectionKind::Function { owner, link: name },
    });

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
        add_type_to_source(&p.param_type, item_provider, context, &mut ret, files);
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
    item_provider: &ItemProvider,
    context: &[NameSymbol],
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    for f in MEMBER_FLAG_ORDER {
        if member.flags.contains(f) {
            ret.add_no_link(member_flag_to_string(f));
            ret.add_no_link(" ");
        }
    }
    add_type_to_source(&member.member_type, item_provider, context, &mut ret, files);
    ret.add_no_link(" ");
    let name = files.text_from_span(member.name.span).to_string();
    ret.add_link(LinkedSection {
        link_prefix: None,
        text: name.clone(),
        kind: LinkedSectionKind::Member { owner, link: name },
    });
    ret
}

fn reconstruct_enumerator_declaration(
    owner: Owner,
    variant: &ir_common::EnumVariant,
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    let name = files.text_from_span(variant.name.span).to_string();
    ret.add_link(LinkedSection {
        link_prefix: None,
        text: name.clone(),
        kind: LinkedSectionKind::Enumerator { owner, link: name },
    });
    if let Some(e) = &variant.init {
        ret.add_no_link(" = ");
        ret.add_no_link(files.text_from_span(e.span.unwrap()));
    }
    ret
}

fn reconstruct_constant_declaration(
    owner: Owner,
    constant: &ir_common::ConstDefinition,
    _item_provider: &ItemProvider,
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    ret.add_no_link("const ");
    let name = files.text_from_span(constant.name.span).to_string();
    ret.add_link(LinkedSection {
        link_prefix: None,
        text: name.clone(),
        kind: LinkedSectionKind::Constant { owner, link: name },
    });
    ret.add_no_link(" = ");
    ret.add_no_link(files.text_from_span(constant.expr.span.unwrap()));
    ret
}

fn reconstruct_static_const_array_declaration(
    owner: Owner,
    sca: &hir::StaticConstArray,
    item_provider: &ItemProvider,
    context: &[NameSymbol],
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    ret.add_no_link("static const ");
    add_type_to_source(&sca.arr_type, item_provider, context, &mut ret, files);
    ret.add_no_link("[] ");
    let name = files.text_from_span(sca.name.span).to_string();
    ret.add_link(LinkedSection {
        link_prefix: None,
        text: name.clone(),
        kind: LinkedSectionKind::Constant { owner, link: name },
    });
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

fn reconstruct_property(
    owner: Owner,
    prop: &ir_common::PropertyDefinition,
    _item_provider: &ItemProvider,
    _context: &[NameSymbol],
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    ret.add_no_link("property ");
    let name = files.text_from_span(prop.name.span);
    ret.add_link(LinkedSection {
        link_prefix: None,
        text: name.to_string(),
        kind: LinkedSectionKind::Property {
            owner,
            link: name.to_string(),
        },
    });
    ret.add_no_link(": ");
    let mut first = true;
    for id in prop.vars.iter() {
        if !first {
            ret.add_no_link(", ");
        }
        first = false;
        ret.add_no_link(files.text_from_span(id.span));
    }
    ret
}

fn reconstruct_flagdef(
    owner: Owner,
    flag: &ir_common::FlagDefinition,
    _item_provider: &ItemProvider,
    _context: &[NameSymbol],
    files: &Files,
) -> SourceCodeWithLinks {
    let mut ret = SourceCodeWithLinks { sections: vec![] };
    ret.add_no_link("flagdef ");
    let name = files.text_from_span(flag.flag_name.span);
    ret.add_link(LinkedSection {
        link_prefix: None,
        text: name.to_string(),
        kind: LinkedSectionKind::Flag {
            owner,
            link: name.to_string(),
        },
    });
    ret.add_no_link(": ");
    ret.add_no_link(files.text_from_span(flag.var_name.span));
    ret.add_no_link(", ");
    ret.add_no_link(files.text_from_span(flag.shift.span));
    ret
}

fn transform_deprecated(d: &hir::Deprecated) -> Deprecated {
    Deprecated {
        version: format!(
            "{}.{}.{}",
            d.version.major, d.version.minor, d.version.revision
        ),
        reason: d
            .message
            .map(|s| s.symbol.string().to_string())
            .unwrap_or_else(|| "".to_string()),
    }
}

fn class_doc(
    name: &str,
    context: &[NameSymbol],
    hir: &hir::TopLevel,
    c: &hir::ClassDefinition,
    files: &Files,
    item_provider: &ItemProvider,
) -> Class {
    let mut class_to_add = Class {
        context: context_with(context, c.name.symbol),
        name: name.to_string(),
        span: c.span,
        inherits: match name {
            "Object" => None,
            _ => {
                let mut source = SourceCodeWithLinks { sections: vec![] };
                match c.ancestor {
                    Some(a) => {
                        add_type_if_possible(
                            files.text_from_span(a.span),
                            Some(a),
                            item_provider,
                            context,
                            false,
                            &mut source,
                        );
                    }
                    None => {
                        add_type_if_possible(
                            "Object",
                            [zscript_parser::interner::intern_name("Object")],
                            item_provider,
                            context,
                            false,
                            &mut source,
                        );
                    }
                }
                Some(source)
            }
        },
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
        properties: vec![],
        flags: vec![],
    };
    for (_, node) in c.inners.iter() {
        let inner_name = files.text_from_span(node[0].name().span);
        match &node[0].kind {
            hir::ClassInnerKind::FunctionDeclaration(f) => {
                if should_skip(f.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Class(vec![name.to_string()]);
                let overrides = if f.flags.contains(hir::FunctionFlags::OVERRIDE) {
                    let mut cur = c;
                    let mut closure = || loop {
                        let ancestor_id = if let Some(a) = cur.ancestor {
                            a
                        } else {
                            break None;
                        };
                        let ancestor = hir.definitions.get(&ancestor_id.symbol)?.iter().find_map(
                            |t| match &t.kind {
                                hir::TopLevelDefinitionKind::Class(c) => Some(c),
                                _ => None,
                            },
                        )?;
                        if let Some(func) = ancestor.inners.get(&f.name.symbol).and_then(|v| {
                            v.iter().find_map(|i| match &i.kind {
                                hir::ClassInnerKind::FunctionDeclaration(f) => Some(f),
                                _ => None,
                            })
                        }) {
                            if func.flags.contains(hir::FunctionFlags::VIRTUAL) {
                                let class_name = files.text_from_span(ancestor.name.span);
                                let func_name = files.text_from_span(func.name.span);
                                break Some(LinkedSection {
                                    link_prefix: None,
                                    text: format!("{}.{}", class_name, func_name),
                                    kind: LinkedSectionKind::Function {
                                        owner: Owner::Class(vec![class_name.to_string()]),
                                        link: func_name.to_string(),
                                    },
                                });
                            }
                        }
                        cur = ancestor;
                    };
                    closure()
                } else {
                    None
                };
                let func_to_add = Function {
                    context: class_to_add.context.clone(),
                    name: inner_name.to_string(),
                    span: f.span,
                    doc_comment: f
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    signature: reconstruct_function_signature(
                        owner,
                        f,
                        item_provider,
                        &class_to_add.context,
                        files,
                    ),
                    overrides,
                    deprecated: f.deprecated.as_ref().map(transform_deprecated),
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
                if should_skip(m.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Class(vec![name.to_string()]);
                let var_to_add = MemberVariable {
                    context: class_to_add.context.clone(),
                    name: inner_name.to_string(),
                    span: m.span,
                    doc_comment: m
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    def: reconstruct_member_declaration(
                        owner,
                        m,
                        item_provider,
                        &class_to_add.context,
                        files,
                    ),
                    deprecated: m.deprecated.as_ref().map(transform_deprecated),
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
                if should_skip(s.doc_comment.as_ref()) {
                    continue;
                }
                let struct_to_add = struct_doc(
                    &format!("{name}.{inner_name}"),
                    inner_name,
                    &class_to_add.context,
                    s,
                    files,
                    item_provider,
                );
                class_to_add.inner_structs.push(struct_to_add);
            }
            hir::ClassInnerKind::Enum(e) => {
                if should_skip(e.doc_comment.as_ref()) {
                    continue;
                }
                let enum_to_add = enum_doc(
                    &format!("{name}.{inner_name}"),
                    inner_name,
                    &class_to_add.context,
                    e,
                    files,
                );
                class_to_add.inner_enums.push(enum_to_add);
            }
            hir::ClassInnerKind::Const(co) => {
                if should_skip(co.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Class(vec![name.to_string()]);
                let const_to_add = Constant {
                    context: class_to_add.context.clone(),
                    name: inner_name.to_string(),
                    doc_comment: co
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: co.span,
                    def: reconstruct_constant_declaration(owner, co, item_provider, files),
                };
                class_to_add.constants.push(const_to_add);
            }
            hir::ClassInnerKind::StaticConstArray(sca) => {
                if should_skip(sca.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Class(vec![name.to_string()]);
                let const_to_add = Constant {
                    context: class_to_add.context.clone(),
                    name: inner_name.to_string(),
                    doc_comment: sca
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: sca.span,
                    def: reconstruct_static_const_array_declaration(
                        owner,
                        sca,
                        item_provider,
                        &class_to_add.context,
                        files,
                    ),
                };
                class_to_add.constants.push(const_to_add);
            }
            hir::ClassInnerKind::Property(p) => {
                if should_skip(p.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Class(vec![name.to_string()]);
                let prop_to_add = Property {
                    context: class_to_add.context.clone(),
                    name: inner_name.to_string(),
                    doc_comment: p
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: p.span,
                    def: reconstruct_property(
                        owner,
                        p,
                        item_provider,
                        &class_to_add.context,
                        files,
                    ),
                };
                class_to_add.properties.push(prop_to_add);
            }
            hir::ClassInnerKind::Flag(f) => {
                if should_skip(f.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Class(vec![name.to_string()]);
                let flag_to_add = Flag {
                    context: class_to_add.context.clone(),
                    name: inner_name.to_string(),
                    doc_comment: f
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: f.span,
                    def: reconstruct_flagdef(owner, f, item_provider, &class_to_add.context, files),
                };
                class_to_add.flags.push(flag_to_add);
            }
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
    class_to_add.overrides.sort_unstable_by_key(|x| x.span);
    class_to_add
        .inner_structs
        .sort_unstable_by(|a, b| a.name.cmp(&b.name));
    class_to_add
        .inner_enums
        .sort_unstable_by(|a, b| a.name.cmp(&b.name));

    class_to_add
}

fn struct_doc(
    name: &str,
    no_context_name: &str,
    context: &[NameSymbol],
    s: &hir::StructDefinition,
    files: &Files,
    item_provider: &ItemProvider,
) -> Struct {
    let mut struct_to_add = Struct {
        context: context_with(context, s.name.symbol),
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
                if should_skip(f.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Struct(vec![name.to_string()]);
                let func_to_add = Function {
                    context: struct_to_add.context.clone(),
                    name: inner_name.to_string(),
                    span: f.span,
                    doc_comment: f
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    signature: reconstruct_function_signature(
                        owner,
                        f,
                        item_provider,
                        &struct_to_add.context,
                        files,
                    ),
                    overrides: None,
                    deprecated: f.deprecated.as_ref().map(transform_deprecated),
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
                if should_skip(m.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Struct(vec![name.to_string()]);
                let var_to_add = MemberVariable {
                    context: struct_to_add.context.clone(),
                    name: inner_name.to_string(),
                    span: m.span,
                    doc_comment: m
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    def: reconstruct_member_declaration(
                        owner,
                        m,
                        item_provider,
                        &struct_to_add.context,
                        files,
                    ),
                    deprecated: m.deprecated.as_ref().map(transform_deprecated),
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
                if should_skip(e.doc_comment.as_ref()) {
                    continue;
                }
                let enum_to_add = enum_doc(
                    &format!("{name}.{inner_name}"),
                    inner_name,
                    &struct_to_add.context,
                    e,
                    files,
                );
                struct_to_add.inner_enums.push(enum_to_add);
            }
            hir::StructInnerKind::Const(c) => {
                if should_skip(c.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Struct(vec![name.to_string()]);
                let const_to_add = Constant {
                    context: struct_to_add.context.clone(),
                    name: inner_name.to_string(),
                    doc_comment: c
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: c.span,
                    def: reconstruct_constant_declaration(owner, c, item_provider, files),
                };
                struct_to_add.constants.push(const_to_add);
            }
            hir::StructInnerKind::StaticConstArray(sca) => {
                if should_skip(sca.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Struct(vec![name.to_string()]);
                let const_to_add = Constant {
                    context: struct_to_add.context.clone(),
                    name: inner_name.to_string(),
                    doc_comment: sca
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: sca.span,
                    def: reconstruct_static_const_array_declaration(
                        owner,
                        sca,
                        item_provider,
                        &struct_to_add.context,
                        files,
                    ),
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
        .inner_enums
        .sort_unstable_by(|a, b| a.name.cmp(&b.name));

    struct_to_add
}

fn enum_doc(
    name: &str,
    no_context_name: &str,
    parent_context: &[NameSymbol],
    e: &ir_common::EnumDefinition,
    files: &Files,
) -> Enum {
    let mut enum_to_add = Enum {
        context: parent_context.to_vec(),
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
        if should_skip(i.doc_comment.as_ref()) {
            continue;
        }
        let inner_name = files.text_from_span(i.name.span);
        let enumerator_to_add = Enumerator {
            context: enum_to_add.context.clone(),
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
    summary_doc: String,
    nice_name: String,
    hir: &hir::TopLevel,
    files: &Files,
    item_provider: &ItemProvider,
    dependencies: &Dependencies,
) -> Documentation {
    let mut docs = Documentation {
        name: nice_name,
        classes: vec![],
        structs: vec![],
        enums: vec![],
        constants: vec![],
        summary_doc,
    };
    for (_, node) in hir.definitions.iter() {
        if node[0].archive_num != dependencies.get_final_archive_num() {
            continue;
        }
        let name = files.text_from_span(node[0].name().span);
        match &node[0].kind {
            hir::TopLevelDefinitionKind::Class(c) => {
                if should_skip(c.doc_comment.as_ref()) {
                    continue;
                }
                let class_to_add = class_doc(name, &[], hir, c, files, item_provider);
                docs.classes.push(class_to_add);
            }
            hir::TopLevelDefinitionKind::Struct(s) => {
                if should_skip(s.doc_comment.as_ref()) {
                    continue;
                }
                let struct_to_add = struct_doc(name, name, &[], s, files, item_provider);
                docs.structs.push(struct_to_add);
            }
            hir::TopLevelDefinitionKind::Enum(e) => {
                if should_skip(e.doc_comment.as_ref()) {
                    continue;
                }
                let enum_to_add = enum_doc(name, name, &[], e, files);
                docs.enums.push(enum_to_add);
            }
            hir::TopLevelDefinitionKind::Const(c) => {
                if should_skip(c.doc_comment.as_ref()) {
                    continue;
                }
                let owner = Owner::Global;
                let const_to_add = Constant {
                    context: vec![],
                    name: name.to_string(),
                    doc_comment: c
                        .doc_comment
                        .map(|s| s.string().to_string())
                        .unwrap_or_else(|| "".to_string()),
                    span: c.span,
                    def: reconstruct_constant_declaration(owner, c, item_provider, files),
                };
                docs.constants.push(const_to_add);
            }
            hir::TopLevelDefinitionKind::MixinClass(_) => {}
        }
    }
    docs.classes.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    docs.structs.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    docs.enums.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    docs.constants.sort_unstable_by_key(|x| x.span);

    docs
}
