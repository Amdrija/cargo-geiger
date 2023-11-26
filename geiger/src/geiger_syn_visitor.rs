#![allow(dead_code)]
use std::path::PathBuf;

use crate::{extern_syn_visitor::RsFileExternDefinitions, ExternCall};

use super::{
    file_forbids_unsafe, has_unsafe_attributes, is_test_fn, is_test_mod,
    IncludeTests, RsFileMetrics,
};

use syn::{visit, Expr, ImplItemMethod, ItemFn, ItemImpl, ItemMod, ItemTrait};

pub struct GeigerSynVisitor<'a> {
    /// Count unsafe usage inside tests
    include_tests: IncludeTests,

    /// The resulting data from a single file scan.
    pub metrics: RsFileMetrics,

    /// The number of nested unsafe scopes that the GeigerSynVisitor are
    /// currently in. For example, if the visitor is inside an unsafe function
    /// and inside an unnecessary unsafe block inside that function, then this
    /// number should be 2. If the visitor is outside unsafe scopes, in a safe
    /// scope, this number should be 0.
    /// This is needed since unsafe scopes can be nested and we need to know
    /// when we leave the outmost unsafe scope and get back into a safe scope.
    unsafe_scopes: u32,

    extern_definitions: &'a RsFileExternDefinitions,

    file: &'a PathBuf,

    current_function: Option<String>,

    package_id: &'a str,
}

impl<'a> GeigerSynVisitor<'a> {
    pub fn new(
        include_tests: IncludeTests,
        extern_definitions: &'a RsFileExternDefinitions,
        file: &'a PathBuf,
        package_id: &'a str,
    ) -> Self {
        GeigerSynVisitor {
            include_tests,
            metrics: Default::default(),
            unsafe_scopes: 0,
            extern_definitions,
            file,
            current_function: None, //we assume that we are in the global scope
            package_id: package_id,
        }
    }

    pub fn enter_unsafe_scope(&mut self) {
        self.unsafe_scopes += 1;
    }

    pub fn exit_unsafe_scope(&mut self) {
        self.unsafe_scopes -= 1;
    }
}

impl<'ast> visit::Visit<'ast> for GeigerSynVisitor<'_> {
    fn visit_file(&mut self, i: &'ast syn::File) {
        self.metrics.forbids_unsafe = file_forbids_unsafe(i);
        syn::visit::visit_file(self, i);
    }

    /// Free-standing functions
    fn visit_item_fn(&mut self, item_fn: &ItemFn) {
        if IncludeTests::No == self.include_tests && is_test_fn(item_fn) {
            return;
        }
        let unsafe_fn =
            item_fn.sig.unsafety.is_some() || has_unsafe_attributes(item_fn);
        if unsafe_fn {
            self.enter_unsafe_scope()
        }
        self.metrics.counters.functions.count(unsafe_fn);

        let before = self.current_function.clone();
        self.current_function = Some(item_fn.sig.ident.to_string());
        visit::visit_item_fn(self, item_fn);
        self.current_function = before;
        if item_fn.sig.unsafety.is_some() {
            self.exit_unsafe_scope()
        }
    }

    fn visit_expr(&mut self, i: &Expr) {
        // Total number of expressions of any type
        match i {
            Expr::Unsafe(i) => {
                self.enter_unsafe_scope();
                visit::visit_expr_unsafe(self, i);
                self.exit_unsafe_scope();
            }
            Expr::Path(_) | Expr::Lit(_) => {
                // Do not count. The expression `f(x)` should count as one
                // expression, not three.
            }
            Expr::Call(call) => {
                if let Expr::Path(path) = call.func.as_ref() {
                    if let Some(ident) = path.path.get_ident() {
                        //TODO: Check why it is not finding strcpy in the hashmap
                        if self
                            .extern_definitions
                            .contains_key(&ident.to_string())
                        {
                            let definition = self
                                .extern_definitions
                                .get(&ident.to_string())
                                .unwrap();

                            self.metrics
                                .extern_calls
                                .entry(definition.clone())
                                .or_default()
                                .push(ExternCall {
                                    extern_definition: definition.clone(),
                                    file: self.file.clone(),
                                    line: ident.span().start().line,
                                    column: ident.span().start().column,
                                    calling_function: self
                                        .current_function
                                        .clone()
                                        .unwrap_or(String::from(
                                            "__global_scope__",
                                        )),
                                    package_id: self.package_id.to_string(),
                                });
                        }
                    }
                }
                self.metrics.counters.exprs.count(self.unsafe_scopes > 0);
                visit::visit_expr_call(self, call);
            }
            other => {
                // TODO: Print something pretty here or gather the data for later
                // printing.
                // if self.verbosity == Verbosity::Verbose && self.unsafe_scopes > 0 {
                //     println!("{:#?}", other);
                // }
                self.metrics.counters.exprs.count(self.unsafe_scopes > 0);
                visit::visit_expr(self, other);
            }
        }
    }

    fn visit_item_mod(&mut self, i: &ItemMod) {
        if IncludeTests::No == self.include_tests && is_test_mod(i) {
            return;
        }
        visit::visit_item_mod(self, i);
    }

    fn visit_item_impl(&mut self, i: &ItemImpl) {
        // unsafe trait impl's
        self.metrics.counters.item_impls.count(i.unsafety.is_some());
        visit::visit_item_impl(self, i);
    }

    fn visit_item_trait(&mut self, i: &ItemTrait) {
        // Unsafe traits
        self.metrics
            .counters
            .item_traits
            .count(i.unsafety.is_some());
        visit::visit_item_trait(self, i);
    }

    fn visit_impl_item_method(&mut self, i: &ImplItemMethod) {
        if i.sig.unsafety.is_some() {
            self.enter_unsafe_scope()
        }
        self.metrics
            .counters
            .methods
            .count(i.sig.unsafety.is_some());
        visit::visit_impl_item_method(self, i);
        if i.sig.unsafety.is_some() {
            self.exit_unsafe_scope()
        }
    }

    // TODO: Visit macros.
    //
    // TODO: Figure out if there are other visit methods that should be
    // implemented here.
}
