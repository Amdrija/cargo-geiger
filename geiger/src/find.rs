use super::{IncludeTests, RsFileMetrics, ScanFileError};

use crate::extern_syn_visitor::{
    ExternSynVisitor, IncludeRustFunctions, RsFileExternDefinitions,
};
use crate::geiger_syn_visitor::GeigerSynVisitor;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

fn load_file(path: &Path) -> Result<String, ScanFileError> {
    let mut file = File::open(path)
        .map_err(|e| ScanFileError::Io(e, path.to_path_buf()))?;
    let mut src = vec![];
    file.read_to_end(&mut src)
        .map_err(|e| ScanFileError::Io(e, path.to_path_buf()))?;
    String::from_utf8(src)
        .map_err(|e| ScanFileError::Utf8(e, path.to_path_buf()))
}

/// Scan a single file for `unsafe` usage.
pub fn find_unsafe_in_file(
    path: &Path,
    include_tests: IncludeTests,
    extern_definitions: &RsFileExternDefinitions,
) -> Result<RsFileMetrics, ScanFileError> {
    let src = load_file(path)?;
    find_unsafe_in_string(
        &src,
        include_tests,
        extern_definitions,
        &path.to_path_buf(),
    )
    .map_err(|e| ScanFileError::Syn(e, path.to_path_buf()))
}

pub fn find_unsafe_in_string(
    src: &str,
    include_tests: IncludeTests,
    extern_definitions: &RsFileExternDefinitions,
    file: &PathBuf,
) -> Result<RsFileMetrics, syn::Error> {
    use syn::visit::Visit;
    let syntax = syn::parse_file(src)?;
    let mut vis =
        GeigerSynVisitor::new(include_tests, extern_definitions, file);
    vis.visit_file(&syntax);
    Ok(vis.metrics)
}

pub fn find_extern_in_file(
    path: &Path,
    include_rust_fns: IncludeRustFunctions,
) -> Result<RsFileExternDefinitions, ScanFileError> {
    let src = load_file(path)?;
    find_extern_in_string(&path.to_path_buf(), &src, include_rust_fns)
        .map_err(|e| ScanFileError::Syn(e, path.to_path_buf()))
}

pub fn find_extern_in_string(
    file_path: &PathBuf,
    src: &str,
    include_rust_fns: IncludeRustFunctions,
) -> Result<RsFileExternDefinitions, syn::Error> {
    use syn::visit::Visit;
    let syntax = syn::parse_file(src)?;
    let mut vis = ExternSynVisitor::new(file_path, include_rust_fns);
    vis.visit_file(&syntax);
    Ok(vis.extern_definitions)
}

#[cfg(test)]
mod find_tests {
    use crate::extern_syn_visitor::ExternDefinition;

    use super::*;

    use cargo_geiger_serde::{Count, CounterBlock};
    use rstest::*;
    use std::{collections::HashMap, io::Write};
    use tempfile::tempdir;

    const FILE_CONTENT_STRING: &str = "use std::io::Write;

pub unsafe fn f() {
    unimplemented!()
}

pub fn g() {
    std::io::stdout().write_all(unsafe {
        std::str::from_utf8_unchecked(b\"binarystring\")
    }.as_bytes()).unwrap();
}

#[no_mangle]
pub fn h() {
    unimplemented!()
}

#[export_name = \"exported_g\"]
pub fn g() {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        unsafe {
            println!(\"Inside unsafe\");
        }
    }
}
";

    #[rstest(
        input_include_tests,
        expected_rs_file_metrics,
        case(
        IncludeTests::Yes,
        RsFileMetrics {
            counters: CounterBlock {
                functions: Count {
                    safe: 2,
                    unsafe_: 3
                },
                exprs: Count {
                    safe: 4,
                    unsafe_: 5
                },
                item_impls: Count {
                    safe: 0,
                    unsafe_: 0
                },
                item_traits: Count {
                    safe: 0,
                    unsafe_: 0
                },
                methods: Count {
                    safe: 0,
                    unsafe_: 0
                }
            },
            forbids_unsafe: false,
            extern_calls: HashMap::new()
        }
        ),
        case(
            IncludeTests::No,
            RsFileMetrics {
                counters: CounterBlock {
                    functions: Count {
                        safe: 1,
                        unsafe_: 3
                    },
                    exprs: Count {
                        safe: 4,
                        unsafe_: 4
                    },
                    item_impls: Count {
                        safe: 0,
                        unsafe_: 0
                    },
                    item_traits: Count {
                        safe: 0,
                        unsafe_: 0
                    },
                    methods: Count {
                        safe: 0,
                        unsafe_: 0
                    }
                },
                forbids_unsafe: false,
                extern_calls: HashMap::new()
            }
        )
    )]
    fn find_unsafe_in_file_test_no_errors(
        input_include_tests: IncludeTests,
        expected_rs_file_metrics: RsFileMetrics,
    ) {
        let temp_dir = tempdir().unwrap();
        let lib_file_path = temp_dir.path().join("lib.rs");
        let mut file = File::create(lib_file_path.clone()).unwrap();

        writeln!(file, "{}", FILE_CONTENT_STRING).unwrap();

        let unsafe_in_file_result = find_unsafe_in_file(
            &lib_file_path,
            input_include_tests,
            &RsFileExternDefinitions::new(),
        );

        assert!(unsafe_in_file_result.is_ok());

        let unsafe_in_file = unsafe_in_file_result.unwrap();

        assert_eq!(unsafe_in_file, expected_rs_file_metrics);
    }

    #[rstest(
        input_include_tests,
        expected_rs_file_metrics,
        case(
            IncludeTests::Yes,
            RsFileMetrics {
                counters: CounterBlock {
                    functions: Count {
                        safe: 2,
                        unsafe_: 3
                    },
                    exprs: Count {
                        safe: 4,
                        unsafe_: 5
                    },
                    item_impls: Count {
                        safe: 0,
                        unsafe_: 0
                    },
                    item_traits: Count {
                        safe: 0,
                        unsafe_: 0
                    },
                    methods: Count {
                        safe: 0,
                        unsafe_: 0
                    }
                },
                forbids_unsafe: false,
                extern_calls: HashMap::new()
            }
        ),
        case(
            IncludeTests::No,
            RsFileMetrics {
                counters: CounterBlock {
                    functions: Count {
                        safe: 1,
                        unsafe_: 3
                    },
                    exprs: Count {
                        safe: 4,
                        unsafe_: 4
                    },
                    item_impls: Count {
                        safe: 0,
                        unsafe_: 0
                    },
                    item_traits: Count {
                        safe: 0,
                        unsafe_: 0
                    },
                    methods: Count {
                        safe: 0,
                        unsafe_: 0
                    }
                },
                forbids_unsafe: false,
                extern_calls: HashMap::new()
            }
        )
    )]
    fn find_unsafe_in_string_test(
        input_include_tests: IncludeTests,
        expected_rs_file_metrics: RsFileMetrics,
    ) {
        let unsafe_in_string_result = find_unsafe_in_string(
            FILE_CONTENT_STRING,
            input_include_tests,
            &RsFileExternDefinitions::new(),
            &PathBuf::from("/test/file_content_string"),
        );

        assert!(unsafe_in_string_result.is_ok());
        let unsafe_in_string = unsafe_in_string_result.unwrap();

        assert_eq!(unsafe_in_string, expected_rs_file_metrics);
    }

    const EXTERN_FILE_CONTENT_STRING: &str = "use std::io::Write;
use libc::{c_int, size_t};

#[link(name = \"snappy\")]
extern \"C\" {
    fn snappy_compress(input: *const u8,
                        input_length: size_t,
                        compressed: *mut u8,
                        compressed_length: *mut size_t) -> c_int;
    fn snappy_uncompress(compressed: *const u8,
                            compressed_length: size_t,
                            uncompressed: *mut u8,
                            uncompressed_length: *mut size_t) -> c_int;
    fn snappy_max_compressed_length(source_length: size_t) -> size_t;
    fn snappy_uncompressed_length(compressed: *const u8,
                                    compressed_length: size_t,
                                    result: *mut size_t) -> c_int;
    fn snappy_validate_compressed_buffer(compressed: *const u8,
                                            compressed_length: size_t) -> c_int;
}

#[link(name = \"test_rust_lib\")]
extern \"Rust\" {
    fn sys_tcp_stream_connect(ip: &[u8], port: u16, timeout: Option<u64>) -> Result<Handle, ()>;
}

#[no_mangle]
pub extern \"C\" fn hello_from_rust() {
    println!(\"Hello from Rust!\");
}

pub unsafe fn f() {
    unimplemented!()
}

pub fn g() {
    std::io::stdout().write_all(unsafe {
        std::str::from_utf8_unchecked(b\"binarystring\")
    }.as_bytes()).unwrap();
}

#[no_mangle]
pub fn h() {
    unimplemented!()
}

#[export_name = \"exported_g\"]
pub fn g() {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[link(name = \"snappy\")]
    extern \"C\" {
        fn foo(asd: size_t) -> size_t;
    }

    #[test]
    #[no_mangle]
    pub extern \"C\" fn bar() {
        println!(\"Hello from Rust!\");
    }

    #[test]
    fn test_1() {
        unsafe {
            println!(\"Inside unsafe\");
        }
    }
}
";

    #[rstest(
        file_path,
        include_rust_fns,
        expected_rs_file_extern_definitions,
        case(
            &PathBuf::from("/test/extern_file_content_string"),
            IncludeRustFunctions::No,
            RsFileExternDefinitions::from([
            (String::from("snappy_compress"), ExternDefinition { file: file_path.clone(),  line: 6, column: 7, name:  String::from("snappy_compress"), contains_pointer_argument: true, args: vec![]}),
            (String::from("snappy_uncompress"), ExternDefinition { file: file_path.clone(), line: 10, column: 7, name: String::from("snappy_uncompress"), contains_pointer_argument: true, args: vec![]  }),
            (String::from("snappy_max_compressed_length"), ExternDefinition { file: file_path.clone(), line: 14, column: 7, name: String::from("snappy_max_compressed_length"), contains_pointer_argument: false, args: vec![]  }),
            (String::from("snappy_uncompressed_length"), ExternDefinition { file: file_path.clone(), line: 15, column: 7, name: String::from("snappy_uncompressed_length"), contains_pointer_argument: true, args: vec![]  }),
            (String::from("snappy_validate_compressed_buffer"), ExternDefinition { file: file_path.clone(),  line: 18, column: 7, name: String::from("snappy_validate_compressed_buffer"), contains_pointer_argument: true, args: vec![] }),
            (String::from("foo"), ExternDefinition { file: file_path.clone(), line: 59, column: 11, name: String::from("foo"), contains_pointer_argument: false, args: vec![]  }),
        ])),
        case(
            &PathBuf::from("/test/extern_file_content_string"),
            IncludeRustFunctions::Yes,
            RsFileExternDefinitions::from([
            (String::from("snappy_compress"), ExternDefinition { file: file_path.clone(),  line: 6, column: 7, name: String::from("snappy_compress"), contains_pointer_argument: true, args: vec![] }),
            (String::from("snappy_uncompress"), ExternDefinition { file: file_path.clone(), line: 10, column: 7, name: String::from("snappy_uncompress"), contains_pointer_argument: true, args: vec![]  }),
            (String::from("snappy_max_compressed_length"), ExternDefinition { file: file_path.clone(),  line: 14, column: 7, name: String::from("snappy_max_compressed_length"), contains_pointer_argument: false, args: vec![]  }),
            (String::from("snappy_uncompressed_length"), ExternDefinition { file: file_path.clone(),  line: 15, column: 7, name: String::from("snappy_uncompressed_length"), contains_pointer_argument: true, args: vec![]  }),
            (String::from("snappy_validate_compressed_buffer"), ExternDefinition { file: file_path.clone(),  line: 18, column: 7, name: String::from("snappy_validate_compressed_buffer"), contains_pointer_argument: true, args: vec![] }),
            (String::from("hello_from_rust"), ExternDefinition { file: file_path.clone(),  line: 28, column: 18, name: String::from("hello_from_rust"), contains_pointer_argument: false, args: vec![]  }),
            (String::from("foo"), ExternDefinition { file: file_path.clone(),  line: 59, column: 11, name: String::from("foo"), contains_pointer_argument: false, args: vec![]  }),
            (String::from("bar"), ExternDefinition { file: file_path.clone(),  line: 64, column: 22, name: String::from("bar"), contains_pointer_argument: false, args: vec![]  }),
        ]))
    )]
    fn find_extern_in_string_test(
        file_path: &PathBuf,
        include_rust_fns: IncludeRustFunctions,
        expected_rs_file_extern_definitions: RsFileExternDefinitions,
    ) {
        let result = find_extern_in_string(
            file_path,
            EXTERN_FILE_CONTENT_STRING,
            include_rust_fns,
        );

        assert!(result.is_ok());

        let result = result.unwrap();

        for (name, extern_definition) in &result {
            println!("{}", name);
            assert!(expected_rs_file_extern_definitions.contains_key(name));
            assert!(
                <std::path::PathBuf as AsRef<std::path::Path>>::as_ref(
                    &expected_rs_file_extern_definitions
                        .get(name)
                        .unwrap()
                        .file
                ) == file_path
            );
            assert!(
                expected_rs_file_extern_definitions.get(name).unwrap().line
                    == extern_definition.line
            );
            assert!(
                expected_rs_file_extern_definitions
                    .get(name)
                    .unwrap()
                    .column
                    == extern_definition.column
            );
            assert!(
                expected_rs_file_extern_definitions
                    .get(name)
                    .unwrap()
                    .contains_pointer_argument
                    == extern_definition.contains_pointer_argument
            );
        }

        for (name, extern_definition) in &expected_rs_file_extern_definitions {
            assert!(result.contains_key(name));
            assert!(
                <std::path::PathBuf as AsRef<std::path::Path>>::as_ref(
                    &result.get(name).unwrap().file
                ) == file_path
            );
            assert!(result.get(name).unwrap().line == extern_definition.line);
            assert!(
                result.get(name).unwrap().column == extern_definition.column
            );
            assert!(
                result.get(name).unwrap().contains_pointer_argument
                    == extern_definition.contains_pointer_argument
            );
        }

        //TODO: The tests dont check the function arguments, as this is extremely cumbersome
    }
}
