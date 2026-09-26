use mpeg_parser::{compile, parse};
use std::fs;

#[test]
fn test_yaml_w_mpeg() {
    let content = fs::read_to_string("examples/yaml_w.mpeg").unwrap();
    let parser = compile(&content).expect("Failed to compile yaml_w.mpeg");
    let valid_yaml = "key1 : value1\nkey2 : 123";
    let res = parse(&parser, valid_yaml);
    assert!(res.is_ok(), "Valid yaml_w should parse! result: {:?}", res);
}

#[test]
fn test_toml_mpeg() {
    let content = fs::read_to_string("examples/toml.mpeg").unwrap();
    let parser = compile(&content).expect("Failed to compile toml.mpeg");
    let valid_toml = "\n    [Section]\n    key = \"value\"\n    key2 = 123\n    arr = [ 1, 2, 3 ]\n    ";
    let res = parse(&parser, valid_toml);
    assert!(res.is_ok(), "Valid toml should parse! result: {:?}", res);
}

#[test]
fn test_ref_rule() {
    let rules = "!W $A(\"a\" $b) $B(\"b\")";
    let parser = compile(rules).expect("Failed to compile ref rule");
    let res = parse(&parser, "a b b");
    assert!(res.is_ok(), "Ref rule should parse! result: {:?}", res);
}
