use std::collections::HashMap;

use zscript_parser::{
    filesystem::Files,
    hir::*,
    interner::NameSymbol,
    ir_common::{ConstDefinition, EnumDefinition},
};

use crate::structures::{LinkedSectionKind, Owner};

pub struct ItemProvider {
    items: HashMap<Vec<NameSymbol>, (String, LinkedSectionKind)>,
}

impl ItemProvider {
    pub fn resolve<T: IntoIterator<Item = NameSymbol> + Clone>(
        &self,
        context: &[NameSymbol],
        chain: T,
    ) -> Option<Vec<(String, LinkedSectionKind)>> {
        let chain_clone = chain.clone();
        let mut chain = chain.into_iter();
        let start = context_with(context, chain.next().unwrap());
        let mut resolved_chain = vec![];
        match self.items.get(&start) {
            Some(i) => {
                resolved_chain.push(i.clone());
                let mut cur = start;
                for next in chain {
                    cur = context_with(&cur, next);
                    match self.items.get(&cur) {
                        Some(i) => {
                            resolved_chain.push(i.clone());
                        }
                        None => {
                            return None;
                        }
                    }
                }
            }
            None => {
                if context.is_empty() {
                    return None;
                } else {
                    return self.resolve(&context[..context.len() - 1], chain_clone);
                }
            }
        }
        Some(resolved_chain)
    }
}

pub trait ToItemProvider {
    fn to_item_provider(&self, files: &Files) -> ItemProvider;
}
impl ToItemProvider for TopLevel {
    fn to_item_provider(&self, files: &Files) -> ItemProvider {
        let mut ret = ItemProvider {
            items: HashMap::new(),
        };
        self.add(&[], &mut ret, files, &Owner::Global);
        ret
    }
}

pub fn context_with(context: &[NameSymbol], with: NameSymbol) -> Vec<NameSymbol> {
    let mut v = context.to_vec();
    v.push(with);
    v
}

fn owner_and(owner: &Owner, and: String) -> Vec<String> {
    let mut base = match owner {
        Owner::Class(v) => v.clone(),
        Owner::Struct(v) => v.clone(),
        Owner::Enum(v) => v.clone(),
        Owner::Global => vec![],
    };
    base.push(and);
    base
}

trait AddToItemProvider {
    fn add(
        &self,
        context: &[NameSymbol],
        item_provider: &mut ItemProvider,
        files: &Files,
        owner: &Owner,
    );
}
impl AddToItemProvider for TopLevel {
    fn add(
        &self,
        context: &[NameSymbol],
        item_provider: &mut ItemProvider,
        files: &Files,
        _owner: &Owner,
    ) {
        for (_, d) in self.definitions.iter() {
            let def = &d[0];
            match &def.kind {
                zscript_parser::hir::TopLevelDefinitionKind::Class(c) => {
                    c.add(context, item_provider, files, &Owner::Global);
                }
                zscript_parser::hir::TopLevelDefinitionKind::Struct(s) => {
                    s.add(context, item_provider, files, &Owner::Global);
                }
                zscript_parser::hir::TopLevelDefinitionKind::Enum(e) => {
                    e.add(context, item_provider, files, &Owner::Global);
                }
                zscript_parser::hir::TopLevelDefinitionKind::Const(c) => {
                    c.add(context, item_provider, files, &Owner::Global);
                }
                zscript_parser::hir::TopLevelDefinitionKind::MixinClass(_) => {}
            }
        }
    }
}

impl AddToItemProvider for ClassDefinition {
    fn add(
        &self,
        context: &[NameSymbol],
        item_provider: &mut ItemProvider,
        files: &Files,
        owner: &Owner,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        let link = owner_and(owner, name.clone());
        let owner = Owner::Class(link.clone());
        item_provider
            .items
            .insert(context.clone(), (name, LinkedSectionKind::Class { link }));
        for (_, d) in self.inners.iter() {
            let def = &d[0];
            match &def.kind {
                ClassInnerKind::FunctionDeclaration(f) => {
                    f.add(&context, item_provider, files, &owner);
                }
                ClassInnerKind::MemberDeclaration(m) => {
                    m.add(&context, item_provider, files, &owner);
                }
                ClassInnerKind::Enum(e) => {
                    e.add(&context, item_provider, files, &owner);
                }
                ClassInnerKind::Struct(s) => {
                    s.add(&context, item_provider, files, &owner);
                }
                ClassInnerKind::Const(co) => {
                    co.add(&context, item_provider, files, &owner);
                }
                ClassInnerKind::StaticConstArray(sca) => {
                    sca.add(&context, item_provider, files, &owner);
                }
                ClassInnerKind::Property(_) => { /* TODO */ }
                ClassInnerKind::Flag(_) => { /* TODO */ }
            }
        }
    }
}

impl AddToItemProvider for StructDefinition {
    fn add(
        &self,
        context: &[NameSymbol],
        item_provider: &mut ItemProvider,
        files: &Files,
        owner: &Owner,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        let link = owner_and(owner, name.clone());
        let owner = Owner::Struct(link.clone());
        item_provider
            .items
            .insert(context.clone(), (name, LinkedSectionKind::Struct { link }));
        for (_, d) in self.inners.iter() {
            let def = &d[0];
            match &def.kind {
                StructInnerKind::FunctionDeclaration(f) => {
                    f.add(&context, item_provider, files, &owner);
                }
                StructInnerKind::MemberDeclaration(m) => {
                    m.add(&context, item_provider, files, &owner);
                }
                StructInnerKind::Enum(e) => {
                    e.add(&context, item_provider, files, &owner);
                }
                StructInnerKind::Const(co) => {
                    co.add(&context, item_provider, files, &owner);
                }
                StructInnerKind::StaticConstArray(sca) => {
                    sca.add(&context, item_provider, files, &owner);
                }
            }
        }
    }
}

impl AddToItemProvider for EnumDefinition {
    fn add(
        &self,
        context: &[NameSymbol],
        item_provider: &mut ItemProvider,
        files: &Files,
        owner: &Owner,
    ) {
        let enum_context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        let link = owner_and(owner, name.clone());
        let owner = Owner::Enum(link.clone());
        item_provider
            .items
            .insert(enum_context, (name, LinkedSectionKind::Enum { link }));
        for v in self.variants.iter() {
            let name = files.text_from_span(v.name.span).to_string();
            item_provider.items.insert(
                context_with(context, v.name.symbol),
                (
                    name.clone(),
                    LinkedSectionKind::Enumerator {
                        owner: owner.clone(),
                        link: name,
                    },
                ),
            );
        }
    }
}

impl AddToItemProvider for ConstDefinition {
    fn add(
        &self,
        context: &[NameSymbol],
        item_provider: &mut ItemProvider,
        files: &Files,
        owner: &Owner,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        item_provider.items.insert(
            context,
            (
                name.to_string(),
                LinkedSectionKind::Constant {
                    owner: owner.clone(),
                    link: name,
                },
            ),
        );
    }
}

impl AddToItemProvider for FunctionDeclaration {
    fn add(
        &self,
        context: &[NameSymbol],
        item_provider: &mut ItemProvider,
        files: &Files,
        owner: &Owner,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        item_provider.items.insert(
            context,
            (
                name.to_string(),
                LinkedSectionKind::Function {
                    owner: owner.clone(),
                    link: name,
                },
            ),
        );
    }
}

impl AddToItemProvider for MemberDeclaration {
    fn add(
        &self,
        context: &[NameSymbol],
        item_provider: &mut ItemProvider,
        files: &Files,
        owner: &Owner,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        item_provider.items.insert(
            context,
            (
                name.to_string(),
                LinkedSectionKind::Member {
                    owner: owner.clone(),
                    link: name,
                },
            ),
        );
    }
}

impl AddToItemProvider for StaticConstArray {
    fn add(
        &self,
        context: &[NameSymbol],
        item_provider: &mut ItemProvider,
        files: &Files,
        owner: &Owner,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        item_provider.items.insert(
            context,
            (
                name.to_string(),
                LinkedSectionKind::Constant {
                    owner: owner.clone(),
                    link: name,
                },
            ),
        );
    }
}
