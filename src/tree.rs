use serde::Deserialize;
use std::sync::LazyLock;

pub const TREE_JSON: &str = include_str!("../Tree/Mipmap Tree.json");

pub static TREE: LazyLock<Tree> =
    LazyLock::new(|| serde_json::from_str(TREE_JSON).expect("tree json must be valid"));

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Item,
    Token,
    Origin,
}

#[derive(Deserialize)]
pub struct Node {
    pub id: u32,
    pub kind: Kind,
    pub category: Option<String>,
    pub requires: Vec<u32>,
}

#[derive(Deserialize)]
pub struct Edge {
    pub from: u32,
    pub to: u32,
}

#[derive(Deserialize)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Tree {
    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }
}
