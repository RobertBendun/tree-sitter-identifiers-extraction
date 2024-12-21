use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query};

fn main() {
    let mut path = None;
    let mut stdin = false;

    let mut args = std::env::args();
    let program_name = args
        .next()
        .expect("why have you called program without argv[0]?!");

    while let Some(arg) = args.next() {
        match &arg[..] {
            "-h" => print_usage_and_exit(&program_name),
            "-i" => stdin = true,
            _ => {
                if path.is_some() {
                    eprintln!("{program_name}: error: expected only one path provided");
                    print_usage_and_exit(&program_name);
                }
                path = Some(arg)
            }
        }
    }

    let mut from_extension = std::collections::HashMap::new();

    let c = (&tree_sitter_cpp::LANGUAGE.into(), "(identifier) @name");
    let cpp = (
        &tree_sitter_cpp::LANGUAGE.into(),
        "(identifier) @name (namespace_identifier) @name",
    );
    let python = (&tree_sitter_python::LANGUAGE.into(), "(identifier) @name");
    let rust = (&tree_sitter_rust::LANGUAGE.into(), "(identifier) @name");

    from_extension.insert("c".to_owned(), c);
    from_extension.insert("cc".to_owned(), cpp);
    from_extension.insert("cpp".to_owned(), cpp);
    from_extension.insert("cxx".to_owned(), cpp);
    from_extension.insert("h".to_owned(), c);
    from_extension.insert("hh".to_owned(), cpp);
    from_extension.insert("hpp".to_owned(), cpp);
    from_extension.insert("hxx".to_owned(), cpp);
    from_extension.insert("py".to_owned(), python);
    from_extension.insert("rs".to_owned(), rust);

    if stdin {
        for line in std::io::stdin().lines() {
            let line = line.unwrap();
            query(&program_name, line.trim(), &from_extension);
        }
    }

    if let Some(path) = &path {
        query(&program_name, path, &from_extension);
    }

    if !stdin && path.is_none() {
        print_usage_and_exit(&program_name);
    }
}

fn query(program_name: &str, path: &str, from_extension: &std::collections::HashMap<String, (&Language, &str)>) {
    if std::path::Path::new(path).is_dir() {
        query_directory(&program_name, path.into(), &from_extension)
    } else {
        query_file(path.into(), &from_extension)
    }
}

fn query_file(
    path: std::path::PathBuf,
    from_extension: &std::collections::HashMap<String, (&Language, &str)>,
) {
    let Some((language, query)) = path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| from_extension.get(ext))
    else {
        // TODO: report an error
        return;
    };

    let mut parser = Parser::new();
    parser
        .set_language(language)
        .expect("Error loading language");
    let query = Query::new(language, query).expect("identifier query");

    let source_code = std::fs::read(path).expect("reading file");
    let tree = parser.parse(&source_code, None).unwrap();
    let mut cursor = tree_sitter::QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source_code.as_slice());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            println!("{}", capture.node.utf8_text(&source_code).unwrap_or(""));
        }
    }
}

fn query_directory(
    program_name: &str,
    path: std::path::PathBuf,
    from_extension: &std::collections::HashMap<String, (&Language, &str)>,
) {
    let mut stack = vec![path];

    while let Some(top) = stack.pop() {
        match std::fs::read_dir(&top) {
            Err(err) => eprintln!("{program_name}: error: read dir: {top:?}: {err}"),
            Ok(dir) => {
                for path in dir {
                    match path {
                        Ok(path) => {
                            if path.file_type().map(|p| p.is_dir()).unwrap_or(false) {
                                stack.push(path.path());
                            } else {
                                query_file(path.path(), from_extension)
                            }
                        }
                        Err(err) => {
                            eprintln!("{program_name}: error: read dir entry: {top:?}: {err}")
                        }
                    }
                }
            }
        }
    }
}

fn print_usage_and_exit(program_name: &str) -> ! {
    eprintln!("usage: {program_name} [-i] [-h] [path]");
    std::process::exit(2)
}
