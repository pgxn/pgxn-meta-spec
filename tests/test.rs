use std::fs::{self, File};
use std::io::{prelude::*, BufReader};
use std::{collections::HashMap, error::Error};

use boon::{Compiler, Schemas};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[test]
fn test_schema_v1() -> Result<(), Box<dyn Error>> {
    let mut compiler = Compiler::new();
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
    // Load the schema and compile the root schema.
    let mut compiler = new_compiler("schema/v1")?;
    const SCHEMA_ID: &str = "https://pgxn.org/meta/v1/distribution.schema.json";
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
    // Load the schema and compile the root schema.
    let mut compiler = new_compiler("schema/v1")?;
    const SCHEMA_ID: &str = "https://pgxn.org/meta/v1/distribution.schema.json";
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
