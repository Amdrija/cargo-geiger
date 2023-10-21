use std::{collections::HashMap, path::PathBuf};

use proc_macro2::LineColumn;
use syn::visit;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternDefinition {
    pub file: PathBuf,
    pub line: LineColumn,
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
        }
    }
}

impl<'ast, 'a> visit::Visit<'ast> for ExternSynVisitor<'a> {
    fn visit_file(&mut self, i: &'ast syn::File) {
        syn::visit::visit_file(self, i);
    }

    //This will visit Rust functions which are marked as extern "C" for calling from C
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if self.include_rust_fns == IncludeRustFunctions::Yes {
            if let Some(_) = &i.sig.abi {
                println!(
                    "{} {:?}",
                    i.sig.ident.to_string(),
                    i.sig.ident.span().start()
                );
                self.extern_definitions.insert(
                    i.sig.ident.to_string(),
                    ExternDefinition {
                        file: self.file.clone(),
                        line: i.sig.ident.span().start(),
                    },
                );
            }
        }

        syn::visit::visit_item_fn(self, i);
    }

    //This will visit the extern block itself
    fn visit_abi(&mut self, i: &'ast syn::Abi) {
        syn::visit::visit_abi(self, i);
    }

    //This will visit the functions coming from C, which reside in the extern "C" {} block.
    fn visit_foreign_item_fn(&mut self, i: &'ast syn::ForeignItemFn) {
        println!(
            "{} {:?}",
            i.sig.ident.to_string(),
            i.sig.ident.span().start()
        );
        self.extern_definitions.insert(
            i.sig.ident.to_string(),
            ExternDefinition {
                file: self.file.clone(),
                line: i.sig.ident.span().start(),
            },
        );

        syn::visit::visit_foreign_item_fn(self, i)
    }
    // TODO: Visit macros.
    //
    // TODO: Figure out if there are other visit methods that should be
    // implemented here.
}
