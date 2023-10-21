use std::{collections::HashMap, path::PathBuf};

use proc_macro2::LineColumn;
use syn::visit;

pub struct ExternDefinition {
    pub file: PathBuf,
    pub line: LineColumn,
}

pub type RsFileExternDefinitions = HashMap<String, ExternDefinition>;

pub struct ExternSynVisitor<'a> {
    file: &'a PathBuf,

    /// The resulting data from a single file scan.
    pub extern_definitions: RsFileExternDefinitions,
}

impl<'a> ExternSynVisitor<'a> {
    pub fn new(file: &'a PathBuf) -> Self {
        ExternSynVisitor {
            file,
            extern_definitions: RsFileExternDefinitions::new(),
        }
    }
}

impl<'ast, 'a> visit::Visit<'ast> for ExternSynVisitor<'a> {
    fn visit_file(&mut self, i: &'ast syn::File) {
        syn::visit::visit_file(self, i);
    }

    fn visit_abi(&mut self, i: &'ast syn::Abi) {
        self.extern_definitions.insert(
            i.name.as_ref().unwrap().value().clone(),
            ExternDefinition {
                file: self.file.clone(),
                line: i.extern_token.span.start(),
            },
        );
    }

    // TODO: Visit macros.
    //
    // TODO: Figure out if there are other visit methods that should be
    // implemented here.
}
