use std::fs::{self, File};
use std::io::{prelude::*, BufReader};
use std::{collections::HashMap, error::Error};

use boon::{Compiler, Schemas};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const SCHEMA_BASE: &str = "https://pgxn.org/meta/v1";
const SCHEMA_ID: &str = "https://pgxn.org/meta/v1/distribution.schema.json";

#[test]
fn test_schema_v1() -> Result<(), Box<dyn Error>> {
    let mut compiler = Compiler::new();
    compiler.enable_format_assertions();
    let mut loaded: HashMap<String, Vec<Value>> = HashMap::new();

    let paths = fs::read_dir("./schema/v1")?;
    for path in paths {
        let path = path?.path();
        let bn = path.file_name().unwrap().to_str().unwrap();
        if bn.ends_with(".schema.json") {
            let schema: Value = serde_json::from_reader(File::open(path.clone())?)?;
            if let Value::String(s) = &schema["$id"] {
                // Make sure that the ID is correct.
                assert_eq!(format!("https://pgxn.org/meta/v1/{bn}"), *s);

                // Add the schema to the compiler.
                compiler.add_resource(s, schema.to_owned())?;

                // Grab the examples, if any, to test later.
                if let Value::Array(a) = &schema["examples"] {
                    loaded.insert(s.clone(), a.to_owned());
                } else {
                    loaded.insert(s.clone(), Vec::new());
                }
            } else {
                panic!("Unable to find ID in {}", path.display());
            }
        } else {
            println!("Skipping {}", path.display());
        }
    }

    // Make sure we found schemas.
    assert!(!loaded.is_empty(), "No schemas loaded!");

    // Make sure each schema we loaded is valid.
    let mut schemas = Schemas::new();
    for (id, examples) in loaded {
        let index = compiler.compile(id.as_str(), &mut schemas)?;
        println!("{} ok", id);

        // Test the schema's examples.
        for (i, example) in examples.iter().enumerate() {
            if let Err(e) = schemas.validate(example, index) {
                panic!("Example {i} failed: {e}");
            }
            // println!("  Example {i} ok");
        }
    }

    Ok(())
}

fn new_compiler(dir: &str) -> Result<Compiler, Box<dyn Error>> {
    let mut compiler = Compiler::new();
    compiler.enable_format_assertions();
    let paths = fs::read_dir(dir)?;
    for path in paths {
        let path = path?.path();
        let bn = path.file_name().unwrap().to_str().unwrap();
        if bn.ends_with(".schema.json") {
            let schema: Value = serde_json::from_reader(File::open(path.clone())?)?;
            if let Value::String(s) = &schema["$id"] {
                // Add the schema to the compiler.
                compiler.add_resource(s, schema.to_owned())?;
            } else {
                panic!("Unable to find ID in {}", path.display());
            }
        } else {
            println!("Skipping {}", path.display());
        }
    }

    Ok(compiler)
}

#[derive(Deserialize, Serialize)]
struct CorpusCase {
    test: String,
    error: Option<String>,
    meta: Value,
}

#[test]
fn test_corpus_v1_valid() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the root schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let index = compiler.compile(SCHEMA_ID, &mut schemas)?;

    // Test each meta JSON in the corpus.
    let file = File::open("tests/corpus/v1/valid.txt")?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let tc: CorpusCase = serde_json::from_str(&line?)?;

        if let Err(e) = schemas.validate(&tc.meta, index) {
            panic!("{} failed: {e}", &tc.test);
        }
        println!("Example {} ok", &tc.test);
    }

    Ok(())
}

#[test]
fn test_corpus_v1_invalid() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the root schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let index = compiler.compile(SCHEMA_ID, &mut schemas)?;

    // Test each meta JSON in the corpus.
    let file = File::open("tests/corpus/v1/invalid.txt")?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let tc: CorpusCase = serde_json::from_str(&line?)?;
        match schemas.validate(&tc.meta, index) {
            Ok(_) => panic!("{} unexpectedly passed!", &tc.test),
            Err(e) => assert!(
                e.to_string().contains(&tc.error.unwrap()),
                "{} error: {e}",
                &tc.test,
            ),
        }
    }

    Ok(())
}

#[test]
fn test_v1_term() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the term schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let id = format!("{SCHEMA_BASE}/term.schema.json");
    let idx = compiler.compile(&id, &mut schemas)?;

    for valid_term in [
        ("two chars", json!("hi")),
        ("underscores", json!("hi_this_is_a_valid_term")),
        ("dashes", json!("hi-this-is-a-valid-term")),
        ("punctuation", json!("!@#$%^&*()-=+{}<>,.?")),
        ("unicode", json!("😀🍒📸")),
    ] {
        if let Err(e) = schemas.validate(&valid_term.1, idx) {
            panic!("extension {} failed: {e}", valid_term.0);
        }
    }

    for invalid_term in [
        ("array", json!([])),
        ("empty string", json!("")),
        ("too short", json!("x")),
        ("true", json!(true)),
        ("false", json!(false)),
        ("null", json!(null)),
        ("object", json!({})),
        ("space", json!("hi there")),
        ("slash", json!("hi/there")),
        ("backslash", json!("hi\\there")),
        ("null byte", json!("hi\x00there")),
    ] {
        if schemas.validate(&invalid_term.1, idx).is_ok() {
            panic!("{} unexpectedly passed!", invalid_term.0)
        }
    }

    Ok(())
}

#[test]
fn test_v1_tags() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the tags schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let id = format!("{SCHEMA_BASE}/tags.schema.json");
    let idx = compiler.compile(&id, &mut schemas)?;

    for valid_tags in [
        ("two chars", json!(["hi"])),
        ("underscores", json!(["hi_this_is_a_valid_tags"])),
        ("dashes", json!(["hi-this-is-a-valid-tags"])),
        ("punctuation", json!(["!@#$%^&*()-=+{}<>,.?"])),
        ("unicode", json!(["😀🍒📸"])),
        ("space", json!(["hi there"])),
        ("multiple", json!(["testing", "json", "😀🍒📸"])),
        ("max length", json!(["x".repeat(255)])),
    ] {
        if let Err(e) = schemas.validate(&valid_tags.1, idx) {
            panic!("extension {} failed: {e}", valid_tags.0);
        }
    }

    for invalid_tags in [
        ("empty array", json!([])),
        ("string", json!("")),
        ("true", json!(true)),
        ("false", json!(false)),
        ("null", json!(null)),
        ("object", json!({})),
        ("true tag", json!([true])),
        ("false tag", json!([false])),
        ("null tag", json!([null])),
        ("object tag", json!([{}])),
        ("empty tag", json!([""])),
        ("too short", json!(["x"])),
        ("object tag", json!({})),
        ("slash", json!(["hi/there"])),
        ("backslash", json!(["hi\\there"])),
        ("null byte", json!(["hi\x00there"])),
        ("too long", json!("x".repeat(256))),
    ] {
        if schemas.validate(&invalid_tags.1, idx).is_ok() {
            panic!("{} unexpectedly passed!", invalid_tags.0)
        }
    }

    Ok(())
}

#[test]
fn test_v1_version() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the version schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let id = format!("{SCHEMA_BASE}/version.schema.json");
    let idx = compiler.compile(&id, &mut schemas)?;

    // https://regex101.com/r/Ly7O1x/3/
    for valid_version in [
        "0.0.4",
        "1.2.3",
        "10.20.30",
        "1.1.2-prerelease+meta",
        "1.1.2+meta",
        "1.1.2+meta-valid",
        "1.0.0-alpha",
        "1.0.0-beta",
        "1.0.0-alpha.beta",
        "1.0.0-alpha.beta.1",
        "1.0.0-alpha.1",
        "1.0.0-alpha0.valid",
        "1.0.0-alpha.0valid",
        "1.0.0-alpha-a.b-c-somethinglong+build.1-aef.1-its-okay",
        "1.0.0-rc.1+build.1",
        "2.0.0-rc.1+build.123",
        "1.2.3-beta",
        "10.2.3-DEV-SNAPSHOT",
        "1.2.3-SNAPSHOT-123",
        "1.0.0",
        "2.0.0",
        "1.1.7",
        "2.0.0+build.1848",
        "2.0.1-alpha.1227",
        "1.0.0-alpha+beta",
        "1.2.3----RC-SNAPSHOT.12.9.1--.12+788",
        "1.2.3----R-S.12.9.1--.12+meta",
        "1.2.3----RC-SNAPSHOT.12.9.1--.12",
        "1.0.0+0.build.1-rc.10000aaa-kk-0.1",
        "1.0.0-0A.is.legal",
    ] {
        let vv = json!(valid_version);
        if let Err(e) = schemas.validate(&vv, idx) {
            panic!("extension {} failed: {e}", valid_version);
        }
    }

    for invalid_version in [
        "1",
        "1.2",
        "1.2.3-0123",
        "1.2.3-0123.0123",
        "1.1.2+.123",
        "+invalid",
        "-invalid",
        "-invalid+invalid",
        "-invalid.01",
        "alpha",
        "alpha.beta",
        "alpha.beta.1",
        "alpha.1",
        "alpha+beta",
        "alpha_beta",
        "alpha.",
        "alpha..",
        "beta",
        "1.0.0-alpha_beta",
        "-alpha.",
        "1.0.0-alpha..",
        "1.0.0-alpha..1",
        "1.0.0-alpha...1",
        "1.0.0-alpha....1",
        "1.0.0-alpha.....1",
        "1.0.0-alpha......1",
        "1.0.0-alpha.......1",
        "01.1.1",
        "1.01.1",
        "1.1.01",
        "1.2",
        "1.2.3.DEV",
        "1.2-SNAPSHOT",
        "1.2.31.2.3----RC-SNAPSHOT.12.09.1--..12+788",
        "1.2-RC-SNAPSHOT",
        "-1.0.3-gamma+b7718",
        "+justmeta",
        "9.8.7+meta+meta",
        "9.8.7-whatever+meta+meta",
        "99999999999999999999999.999999999999999999.99999999999999999----RC-SNAPSHOT.12.09.1--------------------------------..12",

    ] {
        let iv = json!(invalid_version);
        if schemas.validate(&iv, idx).is_ok() {
            panic!("{} unexpectedly passed!", invalid_version)
        }
    }

    Ok(())
}

#[test]
fn test_v1_version_range() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the version_range schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let id = format!("{SCHEMA_BASE}/version_range.schema.json");
    let idx = compiler.compile(&id, &mut schemas)?;

    // https://regex101.com/r/Ly7O1x/3/
    for valid_version in [
        "0.0.4",
        "1.2.3",
        "10.20.30",
        "1.1.2-prerelease+meta",
        "1.1.2+meta",
        "1.1.2+meta-valid",
        "1.0.0-alpha",
        "1.0.0-beta",
        "1.0.0-alpha.beta",
        "1.0.0-alpha.beta.1",
        "1.0.0-alpha.1",
        "1.0.0-alpha0.valid",
        "1.0.0-alpha.0valid",
        "1.0.0-alpha-a.b-c-somethinglong+build.1-aef.1-its-okay",
        "1.0.0-rc.1+build.1",
        "2.0.0-rc.1+build.123",
        "1.2.3-beta",
        "10.2.3-DEV-SNAPSHOT",
        "1.2.3-SNAPSHOT-123",
        "1.0.0",
        "2.0.0",
        "1.1.7",
        "2.0.0+build.1848",
        "2.0.1-alpha.1227",
        "1.0.0-alpha+beta",
        "1.2.3----RC-SNAPSHOT.12.9.1--.12+788",
        "1.2.3----R-S.12.9.1--.12+meta",
        "1.2.3----RC-SNAPSHOT.12.9.1--.12",
        "1.0.0+0.build.1-rc.10000aaa-kk-0.1",
        "1.0.0-0A.is.legal",
    ] {
        for op in ["", "==", "!=", ">", "<", ">=", "<="] {
            for append in [
                "",
                ",<= 1.1.2+meta",
                ",>= 2.0.0, 1.5.6",
                ", >1.2.0, != 12.0.0, < 19.2.0",
            ] {
                let range = json!(format!("{}{}{}", op, valid_version, append));
                if let Err(e) = schemas.validate(&range, idx) {
                    panic!("extension {} failed: {e}", range);
                }
            }
        }

        for bad_op in ["!", "=", "<>", "=>", "=<"] {
            let range = json!(format!("{}{}", bad_op, valid_version));
            if schemas.validate(&range, idx).is_ok() {
                panic!("{} unexpectedly passed!", range)
            }
        }
    }

    for invalid_version in [
        "1",
        "1.2",
        "1.2.3-0123",
        "1.2.3-0123.0123",
        "1.1.2+.123",
        "+invalid",
        "-invalid",
        "-invalid+invalid",
        "-invalid.01",
        "alpha",
        "alpha.beta",
        "alpha.beta.1",
        "alpha.1",
        "alpha+beta",
        "alpha_beta",
        "alpha.",
        "alpha..",
        "beta",
        "1.0.0-alpha_beta",
        "-alpha.",
        "1.0.0-alpha..",
        "1.0.0-alpha..1",
        "1.0.0-alpha...1",
        "1.0.0-alpha....1",
        "1.0.0-alpha.....1",
        "1.0.0-alpha......1",
        "1.0.0-alpha.......1",
        "01.1.1",
        "1.01.1",
        "1.1.01",
        "1.2",
        "1.2.3.DEV",
        "1.2-SNAPSHOT",
        "1.2.31.2.3----RC-SNAPSHOT.12.09.1--..12+788",
        "1.2-RC-SNAPSHOT",
        "-1.0.3-gamma+b7718",
        "+justmeta",
        "9.8.7+meta+meta",
        "9.8.7-whatever+meta+meta",
        "99999999999999999999999.999999999999999999.99999999999999999----RC-SNAPSHOT.12.09.1--------------------------------..12",

    ] {
        let iv = json!(invalid_version);
        if schemas.validate(&iv, idx).is_ok() {
            panic!("{} unexpectedly passed!", invalid_version)
        }
    }

    Ok(())
}

#[test]
fn test_v1_license() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the license schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let id = format!("{SCHEMA_BASE}/license.schema.json");
    let idx = compiler.compile(&id, &mut schemas)?;

    // Test valid license values.
    for valid_license in [
        json!("agpl_3"),
        json!("apache_1_1"),
        json!("apache_2_0"),
        json!("artistic_1"),
        json!("artistic_2"),
        json!("bsd"),
        json!("freebsd"),
        json!("gfdl_1_2"),
        json!("gfdl_1_3"),
        json!("gpl_1"),
        json!("gpl_2"),
        json!("gpl_3"),
        json!("lgpl_2_1"),
        json!("lgpl_3_0"),
        json!("mit"),
        json!("mozilla_1_0"),
        json!("mozilla_1_1"),
        json!("openssl"),
        json!("perl_5"),
        json!("postgresql"),
        json!("qpl_1_0"),
        json!("ssleay"),
        json!("sun"),
        json!("zlib"),
        json!("open_source"),
        json!("restricted"),
        json!("unrestricted"),
        json!("unknown"),
        json!(["postgresql", "perl_5"]),
        json!({"foo": "https://foo.com"}),
        json!({"foo": "https://foo.com", "bar": "https://bar.com"}),
    ] {
        if let Err(e) = schemas.validate(&valid_license, idx) {
            panic!("license {} failed: {e}", valid_license);
        }
    }

    // Test invalid license values.
    for invalid_license in [
        json!("nonesuch"),
        json!("crank"),
        json!(""),
        json!(true),
        json!(false),
        json!(null),
        json!(["nonesuch"]),
        json!([]),
        json!({}),
        json!({"foo": ":hello"}),
    ] {
        if schemas.validate(&invalid_license, idx).is_ok() {
            panic!("{} unexpectedly passed!", invalid_license)
        }
    }

    Ok(())
}

#[test]
fn test_v1_provides() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the provides schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let id = format!("{SCHEMA_BASE}/provides.schema.json");
    let idx = compiler.compile(&id, &mut schemas)?;

    for valid_provides in [
        (
            "required fields",
            json!({"pgtap": {
                "file": "widget.sql",
                "version": "0.26.0",
            }}),
        ),
        (
            "all fields",
            json!({"pgtap": {
                "docfile": "foo/bar.txt",
                "abstract": "This and that",
                "file": "widget.sql",
                "version": "0.26.0",
            }}),
        ),
        (
            "x field",
            json!({"pgtap": {
                "file": "widget.sql",
                "version": "0.26.0",
                "x_foo": 1,
            }}),
        ),
        (
            "X field",
            json!({"pgtap": {
                "file": "widget.sql",
                "version": "0.26.0",
                "X_foo": 1,
            }}),
        ),
        (
            "two extensions",
            json!({
                "pgtap": {
                    "file": "widget.sql",
                    "version": "0.26.0",
                },
                "pgtap_common": {
                    "file": "common.sql",
                    "version": "0.26.0",
                },
            }),
        ),
    ] {
        if let Err(e) = schemas.validate(&valid_provides.1, idx) {
            panic!("{} failed: {e}", valid_provides.0);
        }
    }

    for invalid_provides in [
        // Basics
        ("array", json!([])),
        ("string", json!("crank")),
        ("empty string", json!("")),
        ("true", json!(true)),
        ("false", json!(false)),
        ("null", json!(null)),
        ("empty object", json!({})),
        (
            "invalid key",
            json!({"x": {"file": "x.sql", "version": "1.0.0"}}),
        ),
        (
            "invalid field",
            json!({"xy": {"file": "x.sql", "version": "1.0.0", "foo": "foo"}}),
        ),
        ("no file", json!({"pgtap": {"version": "0.26.0"}})),
        (
            "invalid version",
            json!({"x": {"file": "x.sql", "version": "1.0"}}),
        ),
        (
            "null file",
            json!({"x": {"file": null, "version": "1.0.0"}}),
        ),
        (
            "bare x_",
            json!({"x": {"file": "x.txt", "version": "1.0.0", "x_": 0}}),
        ),
    ] {
        if schemas.validate(&invalid_provides.1, idx).is_ok() {
            panic!("{} unexpectedly passed!", invalid_provides.0)
        }
    }

    Ok(())
}

#[test]
fn test_v1_extension() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the extension schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let id = format!("{SCHEMA_BASE}/extension.schema.json");
    let idx = compiler.compile(&id, &mut schemas)?;

    for valid_extension in [
        (
            "required fields",
            json!( {
                "file": "widget.sql",
                "version": "0.26.0",
            }),
        ),
        (
            "with abstract",
            json!( {
                "file": "widget.sql",
                "version": "0.26.0",
                "abstract": "This and that",
            }),
        ),
        (
            "all fields",
            json!({
                "docfile": "foo/bar.txt",
                "abstract": "This and that",
                "file": "widget.sql",
                "version": "0.26.0",
            }),
        ),
        (
            "x field",
            json!({
                "version": "0.26.0",
                "file": "widget.sql",
                "x_hi": true,
            }),
        ),
        (
            "X field",
            json!({
                "version": "0.26.0",
                "file": "widget.sql",
                "X_bar": 42,
            }),
        ),
    ] {
        if let Err(e) = schemas.validate(&valid_extension.1, idx) {
            panic!("extension {} failed: {e}", valid_extension.0);
        }
    }

    for invalid_extension in [
        // Basics
        ("array", json!([])),
        ("string", json!("crank")),
        ("empty string", json!("")),
        ("true", json!(true)),
        ("false", json!(false)),
        ("null", json!(null)),
        ("empty object", json!({})),
        (
            "invalid field",
            json!({"file": "widget.sql", "version": "0.26.0", "foo": "hi", }),
        ),
        (
            "bare x_",
            json!( {
                "file": "widget.sql",
                "version": "0.26.0",
                "x_": "hi",
            }),
        ),
        // File
        ("no file", json!({"version": "0.26.0"})),
        ("null file", json!({"file": null, "version": "0.26.0"})),
        (
            "empty string file",
            json!({"file": "", "version": "0.26.0"}),
        ),
        ("number file", json!({"file": 42, "version": "0.26.0"})),
        ("bool file", json!({"file": true, "version": "0.26.0"})),
        ("array file", json!({"file": ["hi"], "version": "0.26.0"})),
        ("object file", json!({"file": {}, "version": "0.26.0"})),
        // Version
        ("no version", json!({"file": "widget.sql"})),
        (
            "invalid version",
            json!({"file": "widget.sql", "version": "1.0"}),
        ),
        (
            "null version",
            json!({"file": "widget.sql", "version": null}),
        ),
        (
            "empty version",
            json!({"file": "widget.sql", "version": ""}),
        ),
        (
            "number version",
            json!({"file": "widget.sql", "version": 42}),
        ),
        (
            "bool version",
            json!({"file": "widget.sql", "version": false}),
        ),
        (
            "array version",
            json!({"file": "widget.sql", "version": ["1.0.0"]}),
        ),
        (
            "objet version",
            json!({"file": "widget.sql", "version": {}}),
        ),
        // Abstract
        (
            "empty abstract",
            json!({"file": "widget.sql", "version": "1.0.0", "abstract": ""}),
        ),
        (
            "null abstract",
            json!({"file": "widget.sql", "version": "1.0.0", "abstract": null}),
        ),
        (
            "number abstract",
            json!({"file": "widget.sql", "version": "1.0.0", "abstract": 42}),
        ),
        (
            "bool abstract",
            json!({"file": "widget.sql", "version": "1.0.0", "abstract": true}),
        ),
        (
            "array abstract",
            json!({"file": "widget.sql", "version": "1.0.0", "abstract": ["hi"]}),
        ),
        (
            "object abstract",
            json!({"file": "widget.sql", "version": "1.0.0", "abstract": {}}),
        ),
        // Docfile
        (
            "empty docfile",
            json!({"file": "widget.sql", "version": "1.0.0", "docfile": ""}),
        ),
        (
            "null docfile",
            json!({"file": "widget.sql", "version": "1.0.0", "docfile": null}),
        ),
        (
            "number docfile",
            json!({"file": "widget.sql", "version": "1.0.0", "docfile": 42}),
        ),
        (
            "bool docfile",
            json!({"file": "widget.sql", "version": "1.0.0", "docfile": true}),
        ),
        (
            "array docfile",
            json!({"file": "widget.sql", "version": "1.0.0", "docfile": ["hi"]}),
        ),
        (
            "object docfile",
            json!({"file": "widget.sql", "version": "1.0.0", "docfile": {}}),
        ),
    ] {
        if schemas.validate(&invalid_extension.1, idx).is_ok() {
            panic!("{} unexpectedly passed!", invalid_extension.0)
        }
    }

    Ok(())
}

#[test]
fn test_v1_maintainer() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the maintainer schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let id = format!("{SCHEMA_BASE}/maintainer.schema.json");
    let idx = compiler.compile(&id, &mut schemas)?;

    for valid_maintainer in [
        ("min length", json!("x")),
        ("min length_array", json!(["x"])),
        (
            "name and email",
            json!("David E. Wheeler <theory@pgxn.org>"),
        ),
        (
            "two names and emails",
            json!([
                "David E. Wheeler <theory@pgxn.org>",
                "Josh Berkus <jberkus@pgxn.org>"
            ]),
        ),
        ("space", json!("hi there")),
        ("slash", json!("hi/there")),
        ("backslash", json!("hi\\there")),
        ("null byte", json!("hi\x00there")),
    ] {
        if let Err(e) = schemas.validate(&valid_maintainer.1, idx) {
            panic!("extension {} failed: {e}", valid_maintainer.0);
        }
    }

    for invalid_maintainer in [
        ("empty array", json!([])),
        ("empty string", json!("")),
        ("empty string in array", json!(["hi", ""])),
        ("true", json!(true)),
        ("false", json!(false)),
        ("null", json!(null)),
        ("object", json!({})),
    ] {
        if schemas.validate(&invalid_maintainer.1, idx).is_ok() {
            panic!("{} unexpectedly passed!", invalid_maintainer.0)
        }
    }

    Ok(())
}

#[test]
fn test_v1_meta_spec() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the maintainer schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let id = format!("{SCHEMA_BASE}/meta-spec.schema.json");
    let idx = compiler.compile(&id, &mut schemas)?;

    for valid_meta_spec in [
        ("version 1.0.0 only", json!({"version": "1.0.0"})),
        ("version 1.0.1 only", json!({"version": "1.0.1"})),
        ("version 1.0.2 only", json!({"version": "1.0.2"})),
        ("version 1.0.99 only", json!({"version": "1.0.99"})),
        ("x key", json!({"version": "1.0.99", "x_y": true})),
        ("X key", json!({"version": "1.0.99", "X_x": true})),
        (
            "version plus URL",
            json!({"version": "1.0.0", "url": "https://pgxn.org/meta/spec.txt"}),
        ),
    ] {
        if let Err(e) = schemas.validate(&valid_meta_spec.1, idx) {
            panic!("extension {} failed: {e}", valid_meta_spec.0);
        }
    }

    for invalid_meta_spec in [
        ("array", json!([])),
        ("string", json!("1.0.0")),
        ("empty string", json!("")),
        ("true", json!(true)),
        ("false", json!(false)),
        ("null", json!(null)),
        ("empty object", json!({})),
        ("unknown field", json!({"version": "1.0.0", "foo": "hi"})),
        ("bare x_", json!({"version": "1.0.0", "x_": "hi"})),
        ("version 1.1.0", json!({"version": "1.1.0"})),
        ("version 2.0.0", json!({"version": "2.0.0"})),
        (
            "no_version",
            json!({"url": "https://pgxn.org/meta/spec.txt"}),
        ),
        (
            "invalid url",
            json!({"version": "1.0.1", "url": "https://pgxn.org/meta/spec.html"}),
        ),
    ] {
        if schemas.validate(&invalid_meta_spec.1, idx).is_ok() {
            panic!("{} unexpectedly passed!", invalid_meta_spec.0)
        }
    }

    Ok(())
}

#[test]
fn test_v1_bugtracker() -> Result<(), Box<dyn Error>> {
    // Load the schemas and compile the maintainer schema.
    let mut compiler = new_compiler("schema/v1")?;
    let mut schemas = Schemas::new();
    let id = format!("{SCHEMA_BASE}/bugtracker.schema.json");
    let idx = compiler.compile(&id, &mut schemas)?;

    for valid_bugtracker in [
        ("web only", json!({"web": "https://foo.com"})),
        ("mailto only", json!({"mailto": "hi@example.com"})),
        (
            "web and mailto",
            json!({"web": "https://foo.com", "mailto": "hi@example.com"}),
        ),
        ("x key", json!({"web": "https://foo.com", "x_q": true})),
        ("X key", json!({"web": "https://foo.com", "X_hi": true})),
    ] {
        if let Err(e) = schemas.validate(&valid_bugtracker.1, idx) {
            panic!("extension {} failed: {e}", valid_bugtracker.0);
        }
    }

    for invalid_bugtracker in [
        ("array", json!([])),
        ("string", json!("web")),
        ("empty string", json!("")),
        ("true", json!(true)),
        ("false", json!(false)),
        ("null", json!(null)),
        ("empty object", json!({})),
        ("unknown field", json!({"web": "https://foo.com", "foo": 0})),
        ("bare x_", json!({"web": "https://foo.com", "x_": 0})),
        ("web array", json!({"web": []})),
        ("web object", json!({"web": {}})),
        ("web bool", json!({"web": true})),
        ("web null", json!({"web": null})),
        ("web number", json!({"web": 52})),
        ("mailto array", json!({"mailto": []})),
        ("mailto object", json!({"mailto": {}})),
        ("mailto bool", json!({"mailto": true})),
        ("mailto null", json!({"mailto": null})),
        ("mailto number", json!({"mailto": 52})),
        ("invalid web url", json!({"web": "3ttp://a.com"})),
    ] {
        if schemas.validate(&invalid_bugtracker.1, idx).is_ok() {
            panic!("{} unexpectedly passed!", invalid_bugtracker.0)
        }
    }

    Ok(())
}
