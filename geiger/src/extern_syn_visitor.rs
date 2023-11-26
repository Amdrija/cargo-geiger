use std::{collections::HashMap, path::PathBuf};

use serde::Serialize;
use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Comma, visit, Abi, FnArg,
    ItemForeignMod, Signature,
};

#[derive(Clone, Hash, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ExternDefinition {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub name: String,
    pub contains_pointer_argument: bool,
    pub args: Vec<String>,
}

fn convert_fn_args_to_vec_type(args: &Punctuated<FnArg, Comma>) -> Vec<String> {
    return args
        .into_iter()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(pat_type) => Some(match pat_type.ty.as_ref() {
                syn::Type::Array(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::BareFn(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::Group(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::ImplTrait(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::Infer(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::Macro(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::Never(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::Paren(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::Path(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::Ptr(t) => t.span().source_text().unwrap_or_default(),
                syn::Type::Reference(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::Slice(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::TraitObject(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::Tuple(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                syn::Type::Verbatim(t) => {
                    t.span().source_text().unwrap_or_default()
                }
                _ => todo!(),
            }),
        })
        .collect();
}

pub type RsFileExternDefinitions = HashMap<String, ExternDefinition>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncludeRustFunctions {
    No,
    Yes,
}

//It doesn't make sense to include the option not to parse tests,
//as exported functions should be used in code or to be exported for use
//in other languages, therefore they should never appear in tests
pub struct ExternSynVisitor<'a> {
    file: &'a PathBuf,

    include_rust_fns: IncludeRustFunctions,

    /// The resulting data from a single file scan.
    pub extern_definitions: RsFileExternDefinitions,

    current_abi: Option<Abi>,
}

impl<'a> ExternSynVisitor<'a> {
    pub fn new(
        file: &'a PathBuf,
        include_rust_fns: IncludeRustFunctions,
    ) -> Self {
        ExternSynVisitor {
            file,
            include_rust_fns,
            extern_definitions: RsFileExternDefinitions::new(),
            current_abi: None,
        }
    }

    pub fn is_not_rust_abi(&self, abi: &Abi) -> bool {
        return abi.name.is_none()
            || abi.name.as_ref().unwrap().value() != "Rust";
    }
}

fn check_arguments_contain_pointer(signature: &Signature) -> bool {
    let mut ptr_argument = false;
    for arg in &signature.inputs {
        if let syn::FnArg::Typed(arg_type) = arg {
            if let syn::Type::Ptr(_) = arg_type.ty.as_ref() {
                ptr_argument = true;
                break;
            }
        }
    }

    return ptr_argument;
}

impl<'ast, 'a> visit::Visit<'ast> for ExternSynVisitor<'a> {
    fn visit_file(&mut self, i: &'ast syn::File) {
        syn::visit::visit_file(self, i);
    }

    //This will visit Rust functions which are marked as extern "C" for calling from C
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if self.include_rust_fns == IncludeRustFunctions::Yes {
            if let Some(abi) = &i.sig.abi {
                if self.is_not_rust_abi(abi) {
                    self.extern_definitions.insert(
                        i.sig.ident.to_string(),
                        ExternDefinition {
                            file: self.file.clone(),
                            line: i.sig.ident.span().start().line,
                            column: i.sig.ident.span().start().column,
                            name: i.sig.ident.to_string(),
                            contains_pointer_argument:
                                check_arguments_contain_pointer(&i.sig),
                            args: convert_fn_args_to_vec_type(&i.sig.inputs),
                        },
                    );
                }
            }
        }

        syn::visit::visit_item_fn(self, i);
    }

    //This will visit the extern block itself
    fn visit_abi(&mut self, i: &'ast syn::Abi) {
        //visit only "C" or nonepecified abis
        let before = self.current_abi.clone();
        self.current_abi = Some(i.clone());
        syn::visit::visit_abi(self, i);
        self.current_abi = before;
    }

    fn visit_item_foreign_mod(&mut self, i: &'ast ItemForeignMod) {
        let before = self.current_abi.clone();
        self.current_abi = Some(i.abi.clone());
        syn::visit::visit_item_foreign_mod(self, i);
        self.current_abi = before;
    }

    //This will visit the functions coming from C, which reside in the extern "C" {} block.
    fn visit_foreign_item_fn(&mut self, i: &'ast syn::ForeignItemFn) {
        if self.current_abi.is_some()
            && self.is_not_rust_abi(&self.current_abi.as_ref().unwrap())
        {
            self.extern_definitions.insert(
                i.sig.ident.to_string(),
                ExternDefinition {
                    file: self.file.clone(),
                    line: i.sig.ident.span().start().line,
                    column: i.sig.ident.span().start().column,
                    name: i.sig.ident.to_string(),
                    contains_pointer_argument: check_arguments_contain_pointer(
                        &i.sig,
                    ),
                    args: convert_fn_args_to_vec_type(&i.sig.inputs),
                },
            );
        }

        syn::visit::visit_foreign_item_fn(self, i)
    }
    // TODO: Visit macros.
    //
    // TODO: Figure out if there are other visit methods that should be
    // implemented here.
}
