#![allow(unused_braces)]

use crate::structures::*;

use crate::item::ItemProvider;
use itertools::Itertools;
use pulldown_cmark::{html, BrokenLink, CowStr, LinkType, Options, Parser};
use typed_html::{
    dom::DOMTree,
    elements::{FlowContent, PhrasingContent},
    html, text,
    types::{Id, SpacedSet},
    unsafe_text,
};
use zscript_parser::interner::intern_name;

pub enum SidebarSection {
    Header { text: String, link: Option<String> },
    Text { text: String, link: String },
}

pub struct SidebarData {
    pub docs_name: String,
    pub title: String,
    pub sections: Vec<SidebarSection>,
}

fn add_zws(text: &str) -> String {
    let mut text = text.to_string();
    for c in ['.', '_'] {
        text = text.replace(&c.to_string(), &format!("\u{200B}{c}"))
    }
    text
}

struct SummaryGridRow<'a> {
    name: String,
    link: String,
    doc_comment: String,
    context: &'a [zscript_parser::interner::NameSymbol],
}

fn render_html_boilerplate(
    title: &str,
    body: Box<dyn FlowContent<String>>,
    sidebar_data: SidebarData,
) -> DOMTree<String> {
    html!(
        <html lang="en-US">
            <head>
                <title> { text!(title) } </title>
                <link rel="icon" type="image/x-icon" href="/favicon.png"/>
                <link rel="stylesheet" href="/main.css"/>
                <script src="main.bundle.js"></script>
                <meta charset="UTF-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
            </head>
            <body>
                <div id="header">
                    <button id="header_button">"☰"</button>
                    <h1 id="header_main_link"><a href="/index.html">
                        { text!(&sidebar_data.docs_name) } " Documentation"
                    </a></h1>
                </div>
                <div id="not_header">
                    { render_sidebar(sidebar_data) }
                    <div id="inner">
                        <div id="search">
                            <input id="search_input" placeholder="Search"/>
                        </div>
                        { body }
                    </div>
                </div>
            </body>
        </html>
    )
}

fn render_doc_vis_toggle_button(
    doc_comment: &str,
    id: &str,
) -> Option<Box<dyn FlowContent<String>>> {
    if doc_comment.trim().is_empty() {
        return None;
    }
    Some(html!(
        <div class=["vis_toggle_wrapper", "end_justify"]>
            <button
                id={ &*format!("{id}.vis_button") }
                class="vis_toggle"
            >"-"</button>
        </div>
    ))
}

fn md_event_map<'a>(
    event: pulldown_cmark::Event<'a>,
    i: Option<(&ItemProvider, &[zscript_parser::interner::NameSymbol])>,
) -> pulldown_cmark::Event<'a> {
    use pulldown_cmark::{Event, Tag};
    let link_process = |ty, link: CowStr<'a>, title| match i {
        Some((item_provider, context)) => match ty {
            LinkType::Inline | LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut => {
                let s = link.to_string();
                let chain = s.split('.').map(|x| intern_name(x.trim()));
                match item_provider.resolve(context, chain) {
                    Some(v) => Tag::Link(ty, v.last().unwrap().get_href().into(), s.into()),
                    None => Tag::Link(ty, link, title),
                }
            }
            _ => Tag::Link(ty, link, title),
        },
        None => Tag::Link(ty, link, title),
    };
    // for now i'm simply not dealing with html in markdown since it requires sanitation and i
    // don't want to do that lol
    match event {
        Event::Html(s) => Event::Text(s),
        Event::Start(Tag::Link(ty, link, title)) => Event::Start(link_process(ty, link, title)),
        Event::End(Tag::Link(ty, link, title)) => Event::End(link_process(ty, link, title)),
        e => e,
    }
}

fn broken_link_callback<'a>(
    b: BrokenLink<'a>,
    item_provider: &ItemProvider,
    context: &[zscript_parser::interner::NameSymbol],
) -> Option<(CowStr<'a>, CowStr<'a>)> {
    match b.link_type {
        LinkType::Shortcut => {
            let s = if b.reference.starts_with('`') && b.reference.ends_with('`') {
                b.reference
                    .strip_prefix('`')
                    .unwrap()
                    .strip_suffix('`')
                    .unwrap()
            } else {
                &b.reference
            }
            .to_string();
            let chain = s.split('.').map(|x| intern_name(x.trim()));
            item_provider
                .resolve(context, chain)
                .map(|v| (v.last().unwrap().get_href().into(), s.into()))
        }
        _ => None,
    }
}

pub fn render_doc_summary(
    text: &str,
    item_provider: &ItemProvider,
    context: &[zscript_parser::interner::NameSymbol],
) -> Option<Box<dyn FlowContent<String>>> {
    if text.trim().is_empty() {
        return None;
    }
    use pulldown_cmark::{Event, Tag};

    fn map<'a>(parser: impl Iterator<Item = Event<'a>>) -> impl Iterator<Item = Event<'a>> {
        struct ScannerState {
            level: usize,
            started: bool,
        }
        parser.scan(
            ScannerState {
                level: 0,
                started: false,
            },
            |state, event| {
                fn should_stop(tag: &Tag) -> bool {
                    matches!(
                        tag,
                        Tag::CodeBlock(..)
                            | Tag::Table(..)
                            | Tag::TableHead
                            | Tag::TableRow
                            | Tag::TableCell
                    )
                }
                fn map_tag(tag: Tag) -> Tag {
                    match tag {
                        t @ (Tag::Paragraph
                        | Tag::BlockQuote
                        | Tag::Item
                        | Tag::Emphasis
                        | Tag::Strong
                        | Tag::Link(..)) => t,
                        _ => Tag::Paragraph,
                    }
                }
                if state.started && state.level == 0 {
                    return None;
                }
                state.started = true;
                match event {
                    Event::Start(t) => {
                        if should_stop(&t) {
                            return None;
                        }
                        state.level += 1;
                        Some(Event::Start(map_tag(t)))
                    }
                    Event::End(t) => {
                        if should_stop(&t) {
                            return None;
                        }
                        state.level -= 1;
                        Some(Event::End(map_tag(t)))
                    }
                    e => Some(e),
                }
            },
        )
    }
    let html_output = {
        let dedented = textwrap::dedent(text);

        let options = Options::ENABLE_TABLES;

        let mut broken_link_callback = |x| broken_link_callback(x, item_provider, context);
        let parser = Parser::new_with_broken_link_callback(
            &dedented,
            options,
            Some(&mut broken_link_callback),
        )
        .map(|x| md_event_map(x, Some((item_provider, context))));

        let parser = map(parser);

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        html_output
    };

    Some(html!(
        <div class="inline_summary">
            { unsafe_text!(html_output) }
        </div>
    ))
}

fn render_doc_comment(
    text: &str,
    large: bool,
    id: &str,
    item_provider: &ItemProvider,
    context: &[zscript_parser::interner::NameSymbol],
) -> Option<Box<dyn FlowContent<String>>> {
    if text.trim().is_empty() {
        return None;
    }

    let html_output = {
        let dedented = textwrap::dedent(text);

        let options = Options::ENABLE_TABLES;

        let mut broken_link_callback = |x| broken_link_callback(x, item_provider, context);
        let parser = Parser::new_with_broken_link_callback(
            &dedented,
            options,
            Some(&mut broken_link_callback),
        )
        .map(|x| md_event_map(x, Some((item_provider, context))));

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        html_output
    };

    Some(html!(
        <div class=[
            if large { "doc_comment_large" } else { "doc_comment" },
            "collapsible",
        ] id={ id }>
            { unsafe_text!(html_output) }
        </div>
    ))
}

impl Owner {
    fn get_href_prelude(&self) -> String {
        match self {
            Self::Class(v) => format!("class.{}.html", v.join(".")),
            Self::Struct(v) => format!("struct.{}.html", v.join(".")),
            Self::Enum(v) => format!("enum.{}.html", v.join(".")),
            Self::Builtin(s) => format!("builtin.{}.html", s),
            Self::Global => "index.html".to_string(),
        }
    }
}

impl LinkedSectionKind {
    fn get_style(&self) -> &'static str {
        match &self {
            LinkedSectionKind::Struct { .. } => "struct",
            LinkedSectionKind::Class { .. } => "class",
            LinkedSectionKind::Enum { .. } => "enum",
            LinkedSectionKind::Builtin { .. } => "builtin",
            LinkedSectionKind::Function { .. } => "function",
            LinkedSectionKind::Member { .. } => "member",
            LinkedSectionKind::Enumerator { .. } => "constant",
            LinkedSectionKind::Constant { .. } => "constant",
            LinkedSectionKind::Property { .. } => "default",
            LinkedSectionKind::Flag { .. } => "default",
        }
    }

    fn get_kind_href(&self) -> String {
        match &self {
            LinkedSectionKind::Struct { link } => format!("/struct.{}.html", link.join(".")),
            LinkedSectionKind::Class { link } => format!("/class.{}.html", link.join(".")),
            LinkedSectionKind::Enum { link } => format!("/enum.{}.html", link.join(".")),
            LinkedSectionKind::Builtin { link } => format!("/builtin.{}.html", link),
            LinkedSectionKind::Function { owner, link } => {
                format!("/{}#function.{}", owner.get_href_prelude(), link)
            }
            LinkedSectionKind::Member { owner, link } => {
                format!("/{}#member.{}", owner.get_href_prelude(), link)
            }
            LinkedSectionKind::Enumerator { owner, link } => {
                format!("/{}#enumerator.{}", owner.get_href_prelude(), link)
            }
            LinkedSectionKind::Constant { owner, link } => {
                format!("/{}#constant.{}", owner.get_href_prelude(), link)
            }
            LinkedSectionKind::Property { owner, link } => {
                format!("/{}#property.{}", owner.get_href_prelude(), link)
            }
            LinkedSectionKind::Flag { owner, link } => {
                format!("/{}#flag.{}", owner.get_href_prelude(), link)
            }
        }
    }
}

impl LinkedSection {
    fn get_style(&self) -> &'static str {
        self.kind.get_style()
    }

    fn get_href(&self) -> String {
        let kind_href = self.kind.get_kind_href();
        let prefix = self.link_prefix.as_deref().unwrap_or_default();
        format!("{}{}", prefix, kind_href)
    }
}
impl SourceCodeSection {
    fn render(&self) -> Box<dyn PhrasingContent<String>> {
        match self {
            SourceCodeSection::NoLink(s) => text!(add_zws(s)),
            SourceCodeSection::Linked(l) => html!(
                <a href={ l.get_href() } class={ l.get_style() }>{ text!(add_zws(&l.text)) }</a>
            ),
            SourceCodeSection::NoNewlineSpacing => text!(" "),
            _ => unreachable!(),
        }
    }
}

impl SourceCodeWithLinks {
    fn evaluate_length(&self) -> usize {
        self.sections
            .iter()
            .map(|s| match s {
                SourceCodeSection::NoLink(s) => s.len(),
                SourceCodeSection::Linked(l) => l.text.len(),
                SourceCodeSection::PotentialNewlineOnly => 0,
                SourceCodeSection::PotentialNewlineIndent => 0,
                SourceCodeSection::NoNewlineSpacing => 1,
            })
            .sum()
    }

    fn render_multiline_section(
        indent: bool,
        sections: &[&SourceCodeSection],
    ) -> Box<dyn FlowContent<String>> {
        let classes: SpacedSet<typed_html::types::Class> = if indent {
            ["source_line", "indent"].try_into().unwrap()
        } else {
            ["source_line", "no_indent"].try_into().unwrap()
        };
        html!(
            <pre class=classes><code>
                {
                    sections
                    .iter()
                    .filter(|s| !matches!(
                        s,
                        SourceCodeSection::NoNewlineSpacing
                    ))
                    .map(|s| s.render())
                }
            </code></pre>
        )
    }

    fn render_singleline_section(sections: &[SourceCodeSection]) -> Box<dyn FlowContent<String>> {
        html!(
            <div class="source">
                <pre class=["source_line", "no_indent"]><code>
                    {
                        sections
                        .iter()
                        .filter(|s| !matches!(
                            s,
                            SourceCodeSection::PotentialNewlineIndent
                            | SourceCodeSection::PotentialNewlineOnly
                        ))
                        .map(|s| s.render())
                    }
                </code></pre>
            </div>
        )
    }

    fn render_with_func(
        &self,
        multiline_func: impl Fn(bool, &[&SourceCodeSection]) -> Box<dyn FlowContent<String>>,
        singleline_func: impl Fn(&[SourceCodeSection]) -> Box<dyn FlowContent<String>>,
    ) -> Box<dyn FlowContent<String>> {
        enum MultilineSection<'a> {
            NonIndented(Vec<&'a SourceCodeSection>),
            Indented(Vec<&'a SourceCodeSection>),
        }
        fn group_multiline_sections(sections: &[SourceCodeSection]) -> Vec<MultilineSection> {
            let mut multiline_sections = vec![];
            let mut cur_multiline_section = vec![];
            let mut indenting = false;
            fn add_sections<'a>(
                indenting: bool,
                multiline_sections: &mut Vec<MultilineSection<'a>>,
                cur_multiline_section: Vec<&'a SourceCodeSection>,
            ) {
                if indenting {
                    multiline_sections.push(MultilineSection::Indented(cur_multiline_section));
                } else {
                    multiline_sections.push(MultilineSection::NonIndented(cur_multiline_section));
                }
            }
            for s in sections.iter() {
                match s {
                    s @ (SourceCodeSection::NoLink(_) | SourceCodeSection::Linked(_)) => {
                        cur_multiline_section.push(s);
                    }
                    SourceCodeSection::PotentialNewlineOnly => {
                        add_sections(
                            indenting,
                            &mut multiline_sections,
                            std::mem::take(&mut cur_multiline_section),
                        );
                        indenting = false;
                    }
                    SourceCodeSection::PotentialNewlineIndent => {
                        add_sections(
                            indenting,
                            &mut multiline_sections,
                            std::mem::take(&mut cur_multiline_section),
                        );
                        indenting = true;
                    }
                    SourceCodeSection::NoNewlineSpacing => {}
                }
            }
            add_sections(
                indenting,
                &mut multiline_sections,
                std::mem::take(&mut cur_multiline_section),
            );
            multiline_sections
        }
        let length = self.evaluate_length();
        let multiline = length > 40;
        if multiline {
            let multiline_sections = group_multiline_sections(&self.sections);
            html!(
                <div class="source">
                {
                    multiline_sections.into_iter().map(|m| match m {
                        MultilineSection::NonIndented(s) => multiline_func(false, &s),
                        MultilineSection::Indented(s) => multiline_func(true, &s),
                    })
                }
                </div>
            )
        } else {
            singleline_func(&self.sections)
        }
    }

    fn render(&self) -> Box<dyn FlowContent<String>> {
        self.render_with_func(
            Self::render_multiline_section,
            Self::render_singleline_section,
        )
    }
}

impl MemberVariable {
    fn render(&self, item_provider: &ItemProvider) -> Box<dyn FlowContent<String>> {
        let docs_id = format!("member.{}.docs", self.name);
        html!(
            <div>
                <div class="doc_row" id={ Id::new(format!("member.{}", self.name)) }>
                    <div class="doc_main">
                        { self.def.render() }
                    </div>
                    { render_doc_vis_toggle_button(&self.doc_comment, &docs_id) }
                </div>
                {
                    self.deprecated.as_ref().map(|d| html!(
                        <div class="info deprecated">
                            <span class="info_icon">"🛇"</span>
                            "deprecated since "
                            { text!(&d.version) }
                            {
                                if !d.reason.is_empty() {
                                    Some(text!(format!(": {}", &d.reason)))
                                } else {
                                    None
                                }
                            }
                        </div>
                    ))
                }
                { render_doc_comment(&self.doc_comment, false, &docs_id, item_provider, &self.context) }
                <hr/>
            </div>
        )
    }
}

impl Function {
    fn render(&self, item_provider: &ItemProvider) -> Box<dyn FlowContent<String>> {
        let docs_id = format!("function.{}.docs", self.name);
        html!(
            <div>
                <div class="doc_row" id={ Id::new(format!("function.{}", self.name)) }>
                    <div class="doc_main">
                        { self.signature.render() }
                    </div>
                    { render_doc_vis_toggle_button(&self.doc_comment, &docs_id) }
                </div>
                {
                    self.deprecated.as_ref().map(|d| html!(
                        <div class="info deprecated">
                            <span class="info_icon">"🛇"</span>
                            "deprecated since "
                            { text!(&d.version) }
                            {
                                if !d.reason.is_empty() {
                                    Some(text!(format!(": {}", &d.reason)))
                                } else {
                                    None
                                }
                            }
                        </div>
                    ))
                }
                {
                    self.overrides.as_ref().map(|o| html!(
                        <div class="info">
                            <span class="info_icon">"ⓘ"</span>
                            "overrides "
                            <code><a href=o.get_href() class=o.get_style()>
                                { text!(&o.text) }
                            </a></code>
                        </div>
                    ))
                }
                { render_doc_comment(&self.doc_comment, false, &docs_id, item_provider, &self.context) }
                <hr/>
            </div>
        )
    }
}

impl Constant {
    fn render(&self, item_provider: &ItemProvider) -> Box<dyn FlowContent<String>> {
        let docs_id = format!("constant.{}.docs", self.name);
        html!(
            <div>
                <div class="doc_row" id={ Id::new(format!("constant.{}", self.name)) }>
                    <div class="doc_main">
                        { self.def.render() }
                    </div>
                    { render_doc_vis_toggle_button(&self.doc_comment, &docs_id) }
                </div>
                { render_doc_comment(&self.doc_comment, false, &docs_id, item_provider, &self.context) }
                <hr/>
            </div>
        )
    }
}

impl Property {
    fn render(&self, item_provider: &ItemProvider) -> Box<dyn FlowContent<String>> {
        let docs_id = format!("property.{}.docs", self.name);
        html!(
            <div>
                <div class="doc_row" id={ Id::new(format!("property.{}", self.name)) }>
                    <div class="doc_main">
                        { self.def.render() }
                    </div>
                    { render_doc_vis_toggle_button(&self.doc_comment, &docs_id) }
                </div>
                { render_doc_comment(&self.doc_comment, false, &docs_id, item_provider, &self.context) }
                <hr/>
            </div>
        )
    }
}

impl Flag {
    fn render(&self, item_provider: &ItemProvider) -> Box<dyn FlowContent<String>> {
        let docs_id = format!("flag.{}.docs", self.name);
        html!(
            <div>
                <div class="doc_row" id={ Id::new(format!("flag.{}", self.name)) }>
                    <div class="doc_main">
                        { self.def.render() }
                    </div>
                    { render_doc_vis_toggle_button(&self.doc_comment, &docs_id) }
                </div>
                { render_doc_comment(&self.doc_comment, false, &docs_id, item_provider, &self.context) }
                <hr/>
            </div>
        )
    }
}

fn render_section_from_slice<'a, T, U: IntoIterator<Item = Box<dyn FlowContent<String>>>>(
    name: &str,
    id: &str,
    group_class: &str,
    slice: &[T],
    collapsed_by_default: bool,
    map: impl FnMut(&T) -> U,
) -> impl Iterator<Item = Box<dyn FlowContent<String>>> + 'a {
    if !slice.is_empty() {
        let all_id = format!("{id}.all");
        let mut section_class = SpacedSet::try_from("collapsible").unwrap();
        if collapsed_by_default {
            section_class.add("collapsed_by_default");
        }
        if !group_class.is_empty() {
            section_class.add(group_class);
        }
        itertools::Either::Left(
            (html!(
                <div class="doc_row">
                    <div class="doc_main">
                        <h1 class="sub_heading" id={ id }>{ text!(name) }</h1>
                    </div>
                    <div class=["vis_toggle_wrapper", "mid_justify"]>
                        <button
                            id={ &*format!("{all_id}.vis_button") }
                            class="vis_toggle"
                        >"Group -"</button>
                    </div>
                </div>
            ) as Box<dyn FlowContent<String>>)
                .into_iter()
                .chain(html!(
                    <div id={ &*all_id } class=section_class>
                        { slice.iter().flat_map(map) }
                    </div>
                ) as Box<dyn FlowContent<String>>),
        )
    } else {
        itertools::Either::Right(None.into_iter())
    }
}

fn render_summary_grid<'a>(
    heading: &str,
    heading_id: &str,
    link_class: &'a str,
    data: &[SummaryGridRow<'a>],
    item_provider: &'a ItemProvider,
) -> impl Iterator<Item = Box<dyn FlowContent<String>>> + 'a {
    render_section_from_slice(heading, heading_id, "summary_grid", data, false, move |c| {
        [
            html!(
                <div class="summary_grid_name">
                    <code>
                        <a href={ c.link.clone() } class={ link_class }>
                            { text!(add_zws(&c.name)) }
                        </a>
                    </code>
                </div>
            ) as Box<dyn FlowContent<String>>,
            html!(
                <div class="summary_doc_summary">
                    { render_doc_summary(&c.doc_comment, item_provider, c.context) }
                </div>
            ) as _,
        ]
        .into_iter()
    })
}

fn render_members_functions_pair<'a>(
    (vis, mf, collapsed_by_default): &(&str, &VariablesAndFunctions, bool),
    item_provider: &'a ItemProvider,
) -> impl IntoIterator<Item = Box<dyn FlowContent<String>>> + 'a {
    (render_section_from_slice(
        &format!("{vis} Member Variables"),
        &format!("{}_members", vis.to_string().to_ascii_lowercase()),
        "",
        &mf.variables,
        *collapsed_by_default,
        |v| v.render(item_provider),
    ))
    .chain(render_section_from_slice(
        &format!("{vis} Functions"),
        &format!("{}_functions", vis.to_string().to_ascii_lowercase()),
        "",
        &mf.functions,
        *collapsed_by_default,
        |v| v.render(item_provider),
    ))
}

fn sidebar_sections_from_slice<'a, T>(
    text: &str,
    link: &str,
    slice: &'a [T],
    map: impl FnMut(&T) -> SidebarSection + 'a,
) -> impl Iterator<Item = SidebarSection> + 'a {
    if !slice.is_empty() {
        itertools::Either::Left(
            Some(SidebarSection::Header {
                text: text.to_string(),
                link: Some(link.to_string()),
            })
            .into_iter()
            .chain(slice.iter().map(map)),
        )
    } else {
        itertools::Either::Right(None.into_iter())
    }
}

fn sidebar_sections_members_functions_pair<'a>(
    (vis, mf): &(&str, &'a VariablesAndFunctions),
) -> impl IntoIterator<Item = SidebarSection> + 'a {
    let vis_lowercase = vis.to_ascii_lowercase();
    (sidebar_sections_from_slice(
        &format!("{vis} Member Variables"),
        &format!("#{vis_lowercase}_members"),
        &mf.variables,
        |v| SidebarSection::Text {
            text: v.name.clone(),
            link: format!("#member.{}", v.name),
        },
    ))
    .chain(sidebar_sections_from_slice(
        &format!("{vis} Functions"),
        &format!("#{vis_lowercase}_functions"),
        &mf.functions,
        |v| SidebarSection::Text {
            text: v.name.clone(),
            link: format!("#function.{}", v.name),
        },
    ))
}

fn render_sidebar(data: SidebarData) -> Box<dyn FlowContent<String>> {
    html!(
        <nav id="sidebar">
            <div id="sidebar_main_link_container">
                <div class="sidebar_padder">
                    <h1 id="sidebar_main_link"><a href="/index.html">
                        { text!(data.docs_name) } " Documentation"
                    </a></h1>
                </div>
                <hr/>
            </div>
            <div class="sidebar_padder">
                <p id="sidebar_summary" class="sidebar_text">
                    { text!(add_zws(&data.title)) }
                </p>
            </div>
            <hr/>
            { data.sections.iter().map(|s| match s {
                SidebarSection::Header { text, link: Some(link) } => html!(
                    <div>
                        <a class="sidebar_header sidebar_clickable" href={ link } title={ text }>
                            { text!(text) }
                        </a>
                        <hr/>
                    </div>
                ) as Box<dyn FlowContent<_>>,
                SidebarSection::Header { text, link: None } => html!(
                    <div>
                        <p class="sidebar_header" title={ text }>{ text!(text) }</p>
                        <hr/>
                    </div>
                ) as Box<dyn FlowContent<_>>,
                SidebarSection::Text { text, link } => html!(
                    <a class="sidebar_link sidebar_clickable" href={ link } title={ text }>
                        { text!(text) }
                    </a>
                ) as Box<dyn FlowContent<_>>
            }) }
        </nav>
    )
}

impl Class {
    pub fn render(&self, docs_name: &str, item_provider: &ItemProvider) -> DOMTree<String> {
        let sections =
            sidebar_sections_from_slice("Constants", "#constants", &self.constants, |v| {
                SidebarSection::Text {
                    text: v.name.clone(),
                    link: format!("#constant.{}", v.name),
                }
            })
            .chain(sidebar_sections_from_slice(
                "Properties",
                "#properties",
                &self.properties,
                |v| SidebarSection::Text {
                    text: v.name.clone(),
                    link: format!("#property.{}", v.name),
                },
            ))
            .chain(sidebar_sections_from_slice(
                "Flags",
                "#flags",
                &self.overrides,
                |v| SidebarSection::Text {
                    text: v.name.clone(),
                    link: format!("#flag.{}", v.name),
                },
            ))
            .chain(
                [
                    ("Public", &self.public),
                    ("Protected", &self.protected),
                    ("Private", &self.private),
                ]
                .iter()
                .flat_map(sidebar_sections_members_functions_pair),
            )
            .chain(sidebar_sections_from_slice(
                "Overrides",
                "#overrides",
                &self.overrides,
                |v| SidebarSection::Text {
                    text: v.name.clone(),
                    link: format!("#function.{}", v.name),
                },
            ))
            .chain(sidebar_sections_from_slice(
                "Inner Structs",
                "#inner_structs",
                &self.inner_structs,
                |v| SidebarSection::Text {
                    text: v.no_context_name.clone(),
                    link: format!("#struct.{}", v.name),
                },
            ))
            .chain(sidebar_sections_from_slice(
                "Inner Enums",
                "#inner_enums",
                &self.inner_enums,
                |v| SidebarSection::Text {
                    text: v.no_context_name.clone(),
                    link: format!("#enum.{}", v.name),
                },
            ))
            .collect_vec();
        let sidebar_data = SidebarData {
            docs_name: docs_name.to_string(),
            title: format!("Class {}", self.name),
            sections,
        };
        let docs_id = &format!("class.{}.docs", self.name);
        render_html_boilerplate(
            &format!("Class {} - {}", self.name, docs_name),
            html!(
                <div>
                    <div class="doc_row">
                        <div class="doc_main">
                            <h1 class="main_heading">
                                "Class "
                                <a href={ format!("/class.{}.html", self.name) } class="class">
                                    { text!(add_zws(&self.name)) }
                                </a>
                            </h1>
                            {
                                self.inherits.as_ref().map(|i| html!(
                                    <div class="inherits">
                                        "inherits from "
                                        {
                                            macro_rules! func {
                                                ($t: ty) => {
                                                    |sections: $t| html!(
                                                        <span>
                                                            {
                                                                sections
                                                                .iter()
                                                                .filter(|s| !matches!(
                                                                    s,
                                                                    SourceCodeSection::PotentialNewlineIndent
                                                                    | SourceCodeSection::PotentialNewlineOnly
                                                                ))
                                                                .map(|s| s.render())
                                                            }
                                                        </span>
                                                    )
                                                }
                                            }
                                            i.render_with_func(
                                                |_, s| func!(&[&_])(s),
                                                func!(&[_])
                                            )
                                        }
                                    </div>
                                ))
                            }
                        </div>
                        { render_doc_vis_toggle_button(&self.doc_comment, docs_id) }
                    </div>
                    <hr/>
                    { render_doc_comment(&self.doc_comment, true, docs_id, item_provider, &self.context) }
                    {
                        render_section_from_slice(
                            "Constants", "constants", "", &self.constants, false,
                            |v| {
                                v.render(item_provider)
                            }
                        ).chain(
                            render_section_from_slice(
                                "Properties", "properties", "", &self.properties, false,
                                |v| {
                                    v.render(item_provider)
                                }
                            )
                        ).chain(
                            render_section_from_slice(
                                "Flags", "flags", "", &self.flags, false,
                                |v| {
                                    v.render(item_provider)
                                }
                            )
                        ).chain(
                            [
                                ("Public", &self.public, false),
                                ("Protected", &self.protected, false),
                                ("Private", &self.private, true),
                            ].iter().flat_map(|x| render_members_functions_pair(x, item_provider))
                        ).chain(
                            render_section_from_slice(
                                "Overrides", "overrides", "", &self.overrides, true,
                                |v| {
                                    v.render(item_provider)
                                }
                            )
                        ).chain(
                            render_summary_grid(
                                "Inner Structs",
                                "inner_structs",
                                "struct",
                                &self.inner_structs.iter().map(|s| SummaryGridRow {
                                    name: s.name.clone(),
                                    link: format!("/struct.{}.html", s.name),
                                    doc_comment: s.doc_comment.clone(),
                                    context: &s.context,
                                }).collect_vec(),
                                item_provider,
                            )
                        ).chain(
                            render_summary_grid(
                                "Inner Enums",
                                "inner_enums",
                                "enum",
                                &self.inner_enums.iter().map(|e| SummaryGridRow {
                                    name: e.name.clone(),
                                    link: format!("/enum.{}.html", e.name),
                                    doc_comment: e.doc_comment.clone(),
                                    context: &e.context,
                                }).collect_vec(),
                                item_provider,
                            )
                        )
                    }
                </div>
            ),
            sidebar_data,
        )
    }
}

impl Struct {
    pub fn render(&self, docs_name: &str, item_provider: &ItemProvider) -> DOMTree<String> {
        let sections =
            sidebar_sections_from_slice("Constants", "#constants", &self.constants, |v| {
                SidebarSection::Text {
                    text: v.name.clone(),
                    link: format!("#constant.{}", v.name),
                }
            })
            .chain(
                [
                    ("Public", &self.public),
                    ("Protected", &self.protected),
                    ("Private", &self.private),
                ]
                .iter()
                .flat_map(sidebar_sections_members_functions_pair),
            )
            .chain(sidebar_sections_from_slice(
                "Inner Enums",
                "#inner_enums",
                &self.inner_enums,
                |v| SidebarSection::Text {
                    text: v.no_context_name.clone(),
                    link: format!("#enum.{}", v.name),
                },
            ))
            .collect_vec();
        let sidebar_data = SidebarData {
            docs_name: docs_name.to_string(),
            title: format!("Struct {}", self.name),
            sections,
        };
        let docs_id = format!("struct.{}.docs", self.name);
        render_html_boilerplate(
            &format!("Struct {} - {}", self.name, docs_name),
            html!(
                <div>
                    <div class="doc_row">
                        <div class="doc_main">
                            <h1 class="main_heading">
                                "Struct "
                                <a href={ format!("/struct.{}.html", self.name) } class="struct">
                                    { text!(add_zws(&self.name)) }
                                </a>
                            </h1>
                        </div>
                        { render_doc_vis_toggle_button(&self.doc_comment, &docs_id) }
                    </div>
                    <hr/>
                    { render_doc_comment(&self.doc_comment, true, &docs_id, item_provider, &self.context) }
                    {
                        render_section_from_slice(
                            "Constants", "constants", "", &self.constants, false,
                            |v| {
                                v.render(item_provider)
                            }
                        ).chain(
                            [
                                ("Public", &self.public, false),
                                ("Protected", &self.protected, false),
                                ("Private", &self.private, true)
                            ].iter().flat_map(|x| render_members_functions_pair(x, item_provider))
                        ).chain(
                            render_summary_grid(
                                "Inner Enums",
                                "inner_enums",
                                "enum",
                                &self.inner_enums.iter().map(|e| SummaryGridRow {
                                    name: e.name.clone(),
                                    link: format!("/enum.{}.html", e.name),
                                    doc_comment: e.doc_comment.clone(),
                                    context: &e.context
                                }).collect_vec(),
                                item_provider
                            )
                        )
                    }
                </div>
            ),
            sidebar_data,
        )
    }
}

impl Builtin {
    pub fn render(&self, docs_name: &str, item_provider: &ItemProvider) -> DOMTree<String> {
        let sections =
            sidebar_sections_from_slice("Constants", "#constants", &self.constants, |v| {
                SidebarSection::Text {
                    text: v.name.clone(),
                    link: format!("#constant.{}", v.name),
                }
            })
            .chain(sidebar_sections_from_slice(
                "Functions",
                "functions",
                &self.functions,
                |v| SidebarSection::Text {
                    text: v.name.clone(),
                    link: format!("#function.{}", v.name),
                },
            ))
            .chain(sidebar_sections_from_slice(
                "Member Variables",
                "members",
                &self.variables,
                |v| SidebarSection::Text {
                    text: v.name.clone(),
                    link: format!("#member.{}", v.name),
                },
            ))
            .collect_vec();
        let sidebar_data = SidebarData {
            docs_name: docs_name.to_string(),
            title: format!("Builtin {}", self.name),
            sections,
        };
        let docs_id = format!("builtin.{}.docs", self.name);
        render_html_boilerplate(
            &format!("Builtin {} - {}", self.name, docs_name),
            html!(
                <div>
                    <div class="doc_row">
                        <div class="doc_main">
                            <h1 class="main_heading">
                                "Builtin "
                                <a href={ format!("/builtin.{}.html", self.name) } class="builtin">
                                    { text!(add_zws(&self.name)) }
                                </a>
                            </h1>
                        </div>
                        { render_doc_vis_toggle_button(&self.doc_comment, &docs_id) }
                    </div>
                    <hr/>
                    { render_doc_comment(&self.doc_comment, true, &docs_id, item_provider, &self.context) }
                    {
                        render_section_from_slice(
                            "Constants", "constants", "", &self.constants, false,
                            |v| {
                                v.render(item_provider)
                            }
                        ).chain(
                            render_section_from_slice(
                                "Functions", "functions", "", &self.functions, false,
                                |v| {
                                    v.render(item_provider)
                                }
                            )
                        ).chain(
                            render_section_from_slice(
                                "Member Variables", "members", "", &self.variables, false,
                                |v| {
                                    v.render(item_provider)
                                }
                            )
                        )
                    }
                </div>
            ),
            sidebar_data,
        )
    }
}

impl Enumerator {
    fn render(&self, item_provider: &ItemProvider) -> Box<dyn FlowContent<String>> {
        let docs_id = format!("enumerator.{}.docs", self.name);
        html!(
            <div>
                <div class="doc_row" id={ Id::new(format!("function.{}", self.name)) }>
                    <div class="doc_main">
                        <div class="doc_row" id={ Id::new(format!("enumerator.{}", self.name)) }>
                            { self.decl.render() }
                        </div>
                    </div>
                    { render_doc_vis_toggle_button(&self.doc_comment, &docs_id) }
                </div>
                { render_doc_comment(&self.doc_comment, false, &docs_id, item_provider, &self.context) }
                <hr/>
            </div>
        )
    }
}

impl Enum {
    pub fn render(&self, docs_name: &str, item_provider: &ItemProvider) -> DOMTree<String> {
        let sections =
            sidebar_sections_from_slice("Enumerators", "#enumerators", &self.enumerators, |v| {
                SidebarSection::Text {
                    text: v.name.clone(),
                    link: format!("#enumerator.{}", v.name),
                }
            })
            .collect_vec();
        let sidebar_data = SidebarData {
            docs_name: docs_name.to_string(),
            title: format!("Enum {}", self.name),
            sections,
        };
        let docs_id = format!("enum.{}.docs", self.name);
        render_html_boilerplate(
            &format!("Enum {} - {}", self.name, docs_name),
            html!(
                <div>
                    <div class="doc_row">
                        <div class="doc_main">
                            <h1 class="main_heading">
                                "Enum "
                                <a href={ format!("/enum.{}.html", self.name) } class="enum">
                                    { text!(add_zws(&self.name)) }
                                </a>
                            </h1>
                            <hr/>
                        </div>
                        { render_doc_vis_toggle_button(&self.doc_comment, &docs_id) }
                    </div>
                    { render_doc_comment(&self.doc_comment, true, &docs_id, item_provider, &self.context) }
                    <h1 class="sub_heading" id="enumerators">"Enumerators"</h1>
                    { self.enumerators.iter().map(|v| v.render(item_provider)) }
                </div>
            ),
            sidebar_data,
        )
    }
}

impl Documentation {
    pub fn render_summary_page(&self, item_provider: &ItemProvider) -> DOMTree<String> {
        let mut sections = vec![SidebarSection::Header {
            text: "Contents".to_string(),
            link: None,
        }];
        if !self.constants.is_empty() {
            sections.push(SidebarSection::Text {
                text: "Constants".to_string(),
                link: "#constants".to_string(),
            });
        }
        if !self.enums.is_empty() {
            sections.push(SidebarSection::Text {
                text: "Builtin Types".to_string(),
                link: "#builtins".to_string(),
            });
        }
        if !self.classes.is_empty() {
            sections.push(SidebarSection::Text {
                text: "Classes".to_string(),
                link: "#classes".to_string(),
            });
        }
        if !self.structs.is_empty() {
            sections.push(SidebarSection::Text {
                text: "Structs".to_string(),
                link: "#structs".to_string(),
            });
        }
        if !self.enums.is_empty() {
            sections.push(SidebarSection::Text {
                text: "Enums".to_string(),
                link: "#enums".to_string(),
            });
        }
        let sidebar_data = SidebarData {
            docs_name: self.name.clone(),
            title: format!("Summary of {}", self.name),
            sections,
        };
        let docs_id = "summary_doc";
        render_html_boilerplate(
            &format!("{0} Documentation - {0}", self.name),
            html!(
                <div>
                    <div class="doc_row">
                        <div class="doc_main">
                            <h1 class="main_heading">
                                <a href="/index.html"> { text!(&self.name) } </a>
                                " Documentation"
                            </h1>
                        </div>
                        { render_doc_vis_toggle_button(&self.summary_doc, docs_id) }
                    </div>
                    <hr/>
                    { render_doc_comment(&self.summary_doc, true, docs_id, item_provider, &[]) }
                    {
                        render_section_from_slice(
                            "Constants", "constants", "", &self.constants, false,
                            |v| {
                                v.render(item_provider)
                            }
                        )
                    }
                    {
                        render_summary_grid(
                            "Builtin Types",
                            "builtins",
                            "builtin",
                            &self.builtins.iter().map(|c| SummaryGridRow {
                                name: c.name.clone(),
                                link: format!("/builtin.{}.html", c.name),
                                doc_comment: c.doc_comment.clone(),
                                context: &c.context,
                            }).collect_vec(),
                            item_provider,
                        )
                    }
                    {
                        render_summary_grid(
                            "Classes",
                            "classes",
                            "class",
                            &self.classes.iter().map(|c| SummaryGridRow {
                                name: c.name.clone(),
                                link: format!("/class.{}.html", c.name),
                                doc_comment: c.doc_comment.clone(),
                                context: &c.context,
                            }).collect_vec(),
                            item_provider,
                        )
                    }
                    {
                        render_summary_grid(
                            "Structs",
                            "structs",
                            "struct",
                            &self.structs.iter().map(|s| SummaryGridRow {
                                name: s.name.clone(),
                                link: format!("/struct.{}.html", s.name),
                                doc_comment: s.doc_comment.clone(),
                                context: &s.context,
                            }).collect_vec(),
                            item_provider,
                        )
                    }
                    {
                        render_summary_grid(
                            "Enums",
                            "enums",
                            "enum",
                            &self.enums.iter().map(|e| SummaryGridRow {
                                name: e.name.clone(),
                                link: format!("/enum.{}.html", e.name),
                                doc_comment: e.doc_comment.clone(),
                                context: &e.context,
                            }).collect_vec(),
                            item_provider,
                        )
                    }
                </div>
            ),
            sidebar_data,
        )
    }
}

pub fn render_from_markdown(
    docs_name: &str,
    name: &str,
    markdown: &str,
    link: &str,
    item_provider: &ItemProvider,
) -> DOMTree<String> {
    let sections = vec![];
    let sidebar_data = SidebarData {
        docs_name: docs_name.to_string(),
        title: name.to_string(),
        sections,
    };
    render_html_boilerplate(
        &format!("{} - {}", name, docs_name),
        html!(
            <div>
                <div class="doc_row">
                    <div class="doc_main">
                        <h1 class="main_heading">
                            <a href={ link }> { text!(name) } </a>
                        </h1>
                    </div>
                </div>
                <hr/>
                { render_doc_comment(markdown, true, "content", item_provider, &[]) }
            </div>
        ),
        sidebar_data,
    )
}
