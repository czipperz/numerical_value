extern crate serde;
extern crate serde_json;

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
struct Nodes {
    nodes: Vec<Node>,
}
#[derive(Deserialize, Debug)]
struct Node {
    key: String,
    value: NodeValue,
    successors: Vec<Successor>,
}
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum NodeValue {
    VariableDeclaration { declarations: Vec<Declaration> },
    VariableAssignment { left: Expression, op: String, right: Expression },
    Comparison { left: Expression, op: String, right: Expression },
    Other,
}
#[derive(Deserialize, Debug)]
pub struct Declaration {
    identifier: String,
    initializer: Expression,
}
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Expression {
    Number(i64),
    Identifier(String),
    Other,
}
#[derive(Deserialize, Debug)]
pub struct Successor {
    key: String,
    value: i64,
}

fn read_file() -> io::Result<String> {
    let mut contents = String::new();
    File::open("../input_file.json")?.read_to_string(&mut contents)?;
    Ok(contents)
}

#[derive(Debug)]
pub struct Graph {
    values: HashMap<String, NodeValue>,
    successors: HashMap<String, Vec<Successor>>,
}
impl Graph {
    fn value_of(&self, s: &str) -> Option<&NodeValue> {
        self.values.get(s)
    }
    fn successors_of(&self, s: &str) -> Option<&Vec<Successor>> {
        self.successors.get(s)
    }
}

pub fn parse() -> io::Result<Graph> {
    parse_contents(read_file()?)
}

pub fn parse_contents(contents: String) -> io::Result<Graph> {
    let nodes: Nodes = serde_json::from_str(&contents)?;
    let mut values = HashMap::new();
    let mut successors = HashMap::new();
    for node in nodes.nodes.into_iter() {
        values.insert(node.key.clone(), node.value);
        successors.insert(node.key, node.successors);
    }
    Ok(Graph { values, successors })
}
