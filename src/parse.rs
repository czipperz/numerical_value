extern crate serde;
extern crate serde_json;

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::collections::HashMap;
use std::fmt;

#[derive(Deserialize, Debug, Clone)]
struct Nodes {
    nodes: Vec<Node>,
}
#[derive(Deserialize, Debug, Clone)]
struct Node {
    key: String,
    value: NodeValue,
    successors: Vec<Successor>,
}
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum NodeValue {
    VariableDeclaration { declarations: Vec<Declaration> },
    VariableAssignment { left: /*Expression, op:*/ String, right: Expression },
    Comparison { left: Expression, op: String, right: Expression },
    Other,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Declaration {
    pub identifier: String,
    pub initializer: Expression,
}
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Expression {
    Binary { left: Box<Expression>, op: String, right: Box<Expression> },
    Number(i64),
    Identifier(String),
    Other,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Successor {
    pub key: String,
    pub value: i64,
}

pub struct Graph {
    values: HashMap<String, NodeValue>,
    successors: HashMap<String, Vec<Successor>>,
    first: String,
}
impl Graph {
    #[cfg(test)]
    pub fn new(values: HashMap<String, NodeValue>,
               successors: HashMap<String, Vec<Successor>>,
               first: String) -> Self {
        Graph { values, successors, first }
    }

    pub fn value_of(&self, s: &str) -> Option<&NodeValue> {
        self.values.get(s)
    }
    pub fn successors_of(&self, s: &str) -> Option<&Vec<Successor>> {
        self.successors.get(s)
    }
    pub fn first(&self) -> &String {
        &self.first
    }
}
impl fmt::Debug for Graph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Graph {{")?;
        for k in self.values.keys() {
            writeln!(f, "  {} ->\n    {:?}", k, self.values.get(k).unwrap())?;
            for s in self.successors.get(k).unwrap() {
                writeln!(f, "    {:?}", s)?;
            }
        }
        writeln!(f, "}}")
    }
}

impl fmt::Display for NodeValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use NodeValue::*;
        match self {
            VariableDeclaration { declarations } => {
                for d in declarations {
                    write!(f, "{};", d)?;
                }
                Ok(())
            },
            VariableAssignment { left, right } =>
                write!(f, "{} = {};", left, right),
            Comparison { left, op, right } =>
                write!(f, "{} {} {}", left, op, right),
            Other =>
                write!(f, "__other_stmt();"),
        }
    }
}

impl fmt::Display for Declaration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "let {} = {}", self.identifier, self.initializer)
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Expression::*;
        match self {
            Binary { left, op, right } =>
                write!(f, "{} {} {}", left, op, right),
            Number(num) =>
                write!(f, "{}", num),
            Identifier(identifier) =>
                write!(f, "{}", identifier),
            Other =>
                write!(f, "__other_expr()"),
        }
    }
}

pub fn parse(path: &str) -> io::Result<Graph> {
    Ok(parse_(serde_json::from_reader(File::open(path)?)?))
}

pub fn parse_contents(contents: String) -> io::Result<Graph> {
    Ok(parse_(serde_json::from_str(&contents)?))
}

fn parse_(nodes: Nodes) -> Graph {
    let mut values = HashMap::new();
    let mut successors = HashMap::new();
    let first = nodes.nodes[0].key.clone();
    for node in nodes.nodes.into_iter() {
        values.insert(node.key.clone(), node.value);
        successors.insert(node.key, node.successors);
    }
    Graph { values, successors, first }
}
