use anyhow::Result;
use swc_core::css::{
    ast::{AtRule, AtRulePrelude, ImportHref, Rule, Stylesheet, Url},
    visit::{VisitMut, VisitMutWith},
};
use turbo_tasks::{Value, Vc};
use turbopack_core::{
    issue::{IssueSeverity, IssueSource},
    reference::ModuleReference,
    reference_type::{CssReferenceSubType, ReferenceType},
    resolve::{
        handle_resolve_error,
        origin::{ResolveOrigin, ResolveOriginExt},
        parse::Request,
        ModuleResolveResult,
    },
    source::Source,
};

use crate::{
    references::{
        import::{ImportAssetReference, ImportAttributes},
        url::UrlAssetReference,
    },
    util::str_of_url,
};

pub(crate) mod compose;
pub(crate) mod import;
pub(crate) mod internal;
pub(crate) mod url;

pub type AnalyzedRefs = (
    Vec<Vc<Box<dyn ModuleReference>>>,
    Vec<(String, Vc<UrlAssetReference>)>,
);

/// Returns `(all_references, urls)`.
pub fn analyze_references(
    stylesheet: &mut Stylesheet,
    source: Vc<Box<dyn Source>>,
    origin: Vc<Box<dyn ResolveOrigin>>,
) -> Result<AnalyzedRefs> {
    let mut references = Vec::new();
    let mut urls = Vec::new();

    let mut visitor = ModuleReferencesVisitor::new(source, origin, &mut references, &mut urls);
    stylesheet.visit_mut_with(&mut visitor);

    Ok((references, urls))
}

struct ModuleReferencesVisitor<'a> {
    source: Vc<Box<dyn Source>>,
    origin: Vc<Box<dyn ResolveOrigin>>,
    references: &'a mut Vec<Vc<Box<dyn ModuleReference>>>,
    urls: &'a mut Vec<(String, Vc<UrlAssetReference>)>,
}

impl<'a> ModuleReferencesVisitor<'a> {
    fn new(
        source: Vc<Box<dyn Source>>,
        origin: Vc<Box<dyn ResolveOrigin>>,
        references: &'a mut Vec<Vc<Box<dyn ModuleReference>>>,
        urls: &'a mut Vec<(String, Vc<UrlAssetReference>)>,
    ) -> Self {
        Self {
            source,
            origin,
            references,
            urls,
        }
    }
}

impl<'a> VisitMut for ModuleReferencesVisitor<'a> {
    fn visit_mut_rule(&mut self, rule: &mut Rule) {
        match rule {
            Rule::AtRule(box AtRule {
                prelude: Some(box AtRulePrelude::ImportPrelude(i)),
                ..
            }) => {
                let src = match &*i.href {
                    ImportHref::Url(u) => str_of_url(u),
                    ImportHref::Str(s) => s.value,
                };

                let issue_span = i.span;

                self.references.push(Vc::upcast(ImportAssetReference::new(
                    self.origin,
                    Request::parse(Value::new(src.to_string().into())),
                    ImportAttributes::new_from_prelude(&i.clone()).into(),
                    IssueSource::from_byte_offset(
                        Vc::upcast(self.source),
                        issue_span.lo.0 as _,
                        issue_span.hi.0 as _,
                    ),
                )));
                let vc = UrlAssetReference::new(
                    self.origin,
                    Request::parse(Value::new(src.to_string().into())),
                    IssueSource::from_byte_offset(
                        Vc::upcast(self.source),
                        issue_span.lo.0 as _,
                        issue_span.hi.0 as _,
                    ),
                );
                self.urls.push((src.to_string(), vc));

                i.visit_mut_children_with(self);
            }

            _ => rule.visit_mut_children_with(self),
        }
    }

    fn visit_mut_url(&mut self, u: &mut Url) {
        let src = &*str_of_url(u);

        // ignore internal urls like `url(#noiseFilter)`
        // ignore server-relative urls like `url(/foo)`
        if !matches!(src.bytes().next(), Some(b'#') | Some(b'/')) {
            let issue_span = u.span;

            let vc = UrlAssetReference::new(
                self.origin,
                Request::parse(Value::new(src.to_string().into())),
                IssueSource::from_byte_offset(
                    Vc::upcast(self.source),
                    issue_span.lo.0 as _,
                    issue_span.hi.0 as _,
                ),
            );

            self.references.push(Vc::upcast(vc));
            self.urls.push((str_of_url(u).to_string(), vc));
        }

        u.visit_mut_children_with(self);
    }
}

#[turbo_tasks::function]
pub async fn css_resolve(
    origin: Vc<Box<dyn ResolveOrigin>>,
    request: Vc<Request>,
    ty: Value<CssReferenceSubType>,
    issue_source: Option<Vc<IssueSource>>,
) -> Result<Vc<ModuleResolveResult>> {
    let ty = Value::new(ReferenceType::Css(ty.into_value()));
    let options = origin.resolve_options(ty.clone());
    let result = origin.resolve_asset(request, options, ty.clone());

    handle_resolve_error(
        result,
        ty,
        origin.origin_path(),
        request,
        options,
        issue_source,
        IssueSeverity::Error.cell(),
    )
    .await
}
