use std::collections::HashMap;

use zscript_parser::{
    filesystem::Files,
    hir::*,
    interner::NameSymbol,
    ir_common::{ConstDefinition, EnumDefinition},
};

use crate::structures::{Dependencies, LinkedSection, LinkedSectionKind, Owner};

pub struct ItemProvider {
    items: HashMap<Vec<NameSymbol>, LinkedSection>,
}

impl ItemProvider {
    pub fn resolve<T: IntoIterator<Item = NameSymbol> + Clone>(
        &self,
        context: &[NameSymbol],
        chain: T,
    ) -> Option<Vec<&LinkedSection>> {
        let chain_clone = chain.clone();
        let mut chain = chain.into_iter();
        let start = context_with(context, chain.next().unwrap());
        let mut resolved_chain = vec![];
        match self.items.get(&start) {
            Some(i) => {
                resolved_chain.push(i);
                let mut cur = start;
                for next in chain {
                    cur = context_with(&cur, next);
                    match self.items.get(&cur) {
                        Some(i) => {
                            resolved_chain.push(i);
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
    fn to_item_provider(&self, files: &Files, dependencies: &Dependencies) -> ItemProvider;
}
impl ToItemProvider for TopLevel {
    fn to_item_provider(&self, files: &Files, dependencies: &Dependencies) -> ItemProvider {
        let mut ret = ItemProvider {
            items: HashMap::new(),
        };
        self.add(&[], &mut ret, files, &Owner::Global, dependencies, 0);
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
        dependencies: &Dependencies,
        archive_num: usize,
    );
}
impl AddToItemProvider for TopLevel {
    fn add(
        &self,
        context: &[NameSymbol],
        item_provider: &mut ItemProvider,
        files: &Files,
        _owner: &Owner,
        dependencies: &Dependencies,
        _archive_num: usize,
    ) {
        for (_, d) in self.definitions.iter() {
            let def = &d[0];
            let archive_num = def.archive_num;
            match &def.kind {
                zscript_parser::hir::TopLevelDefinitionKind::Class(c) => {
                    c.add(
                        context,
                        item_provider,
                        files,
                        &Owner::Global,
                        dependencies,
                        archive_num,
                    );
                }
                zscript_parser::hir::TopLevelDefinitionKind::Struct(s) => {
                    s.add(
                        context,
                        item_provider,
                        files,
                        &Owner::Global,
                        dependencies,
                        archive_num,
                    );
                }
                zscript_parser::hir::TopLevelDefinitionKind::Enum(e) => {
                    e.add(
                        context,
                        item_provider,
                        files,
                        &Owner::Global,
                        dependencies,
                        archive_num,
                    );
                }
                zscript_parser::hir::TopLevelDefinitionKind::Const(c) => {
                    c.add(
                        context,
                        item_provider,
                        files,
                        &Owner::Global,
                        dependencies,
                        archive_num,
                    );
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
        dependencies: &Dependencies,
        archive_num: usize,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        let link = owner_and(owner, name.clone());
        let owner = Owner::Class(link.clone());
        item_provider.items.insert(
            context.clone(),
            LinkedSection {
                link_prefix: dependencies.get_link_prefix(archive_num),
                text: name,
                kind: LinkedSectionKind::Class { link },
            },
        );
        for (_, d) in self.inners.iter() {
            let def = &d[0];
            match &def.kind {
                ClassInnerKind::FunctionDeclaration(f) => {
                    f.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
                }
                ClassInnerKind::MemberDeclaration(m) => {
                    m.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
                }
                ClassInnerKind::Enum(e) => {
                    e.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
                }
                ClassInnerKind::Struct(s) => {
                    s.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
                }
                ClassInnerKind::Const(co) => {
                    co.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
                }
                ClassInnerKind::StaticConstArray(sca) => {
                    sca.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
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
        dependencies: &Dependencies,
        archive_num: usize,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        let link = owner_and(owner, name.clone());
        let owner = Owner::Struct(link.clone());
        item_provider.items.insert(
            context.clone(),
            LinkedSection {
                link_prefix: dependencies.get_link_prefix(archive_num),
                text: name,
                kind: LinkedSectionKind::Struct { link },
            },
        );
        for (_, d) in self.inners.iter() {
            let def = &d[0];
            match &def.kind {
                StructInnerKind::FunctionDeclaration(f) => {
                    f.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
                }
                StructInnerKind::MemberDeclaration(m) => {
                    m.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
                }
                StructInnerKind::Enum(e) => {
                    e.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
                }
                StructInnerKind::Const(co) => {
                    co.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
                }
                StructInnerKind::StaticConstArray(sca) => {
                    sca.add(
                        &context,
                        item_provider,
                        files,
                        &owner,
                        dependencies,
                        archive_num,
                    );
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
        dependencies: &Dependencies,
        archive_num: usize,
    ) {
        let enum_context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        let link = owner_and(owner, name.clone());
        let owner = Owner::Enum(link.clone());
        item_provider.items.insert(
            enum_context,
            LinkedSection {
                link_prefix: dependencies.get_link_prefix(archive_num),
                text: name,
                kind: LinkedSectionKind::Enum { link },
            },
        );
        for v in self.variants.iter() {
            let name = files.text_from_span(v.name.span).to_string();
            item_provider.items.insert(
                context_with(context, v.name.symbol),
                LinkedSection {
                    link_prefix: dependencies.get_link_prefix(archive_num),
                    text: name.clone(),
                    kind: LinkedSectionKind::Enumerator {
                        owner: owner.clone(),
                        link: name,
                    },
                },
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
        dependencies: &Dependencies,
        archive_num: usize,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        item_provider.items.insert(
            context,
            LinkedSection {
                link_prefix: dependencies.get_link_prefix(archive_num),
                text: name.to_string(),
                kind: LinkedSectionKind::Constant {
                    owner: owner.clone(),
                    link: name,
                },
            },
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
        dependencies: &Dependencies,
        archive_num: usize,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        item_provider.items.insert(
            context,
            LinkedSection {
                link_prefix: dependencies.get_link_prefix(archive_num),
                text: name.to_string(),
                kind: LinkedSectionKind::Function {
                    owner: owner.clone(),
                    link: name,
                },
            },
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
        dependencies: &Dependencies,
        archive_num: usize,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        item_provider.items.insert(
            context,
            LinkedSection {
                link_prefix: dependencies.get_link_prefix(archive_num),
                text: name.to_string(),
                kind: LinkedSectionKind::Member {
                    owner: owner.clone(),
                    link: name,
                },
            },
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
        dependencies: &Dependencies,
        archive_num: usize,
    ) {
        let context = context_with(context, self.name.symbol);
        let name = files.text_from_span(self.name.span).to_string();
        item_provider.items.insert(
            context,
            LinkedSection {
                link_prefix: dependencies.get_link_prefix(archive_num),
                text: name.to_string(),
                kind: LinkedSectionKind::Constant {
                    owner: owner.clone(),
                    link: name,
                },
            },
        );
    }
}
