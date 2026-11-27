use actix_web::web;
use serde::Deserialize;
use std::sync::LazyLock;

mod images;
mod layout;
mod progress;

pub use images::{item_image, spawn as spawn_images};
pub use layout::LAYOUT_JSON;
pub use progress::{backfill_prices, refund, unlocked};

pub const STICKER: u32 = 53;
pub const STICKER_TOKEN: u32 = 54;

pub fn tutorial(id: u32) -> bool {
    id == STICKER || id == STICKER_TOKEN
}

pub fn tutorial_mips() -> i32 {
    TREE.nodes
        .iter()
        .filter(|node| tutorial(node.id))
        .map(Node::cost)
        .sum()
}

const TREE_JSON: &str = include_str!("../../Tree/Mipmap Tree.json");
const DISCOUNT_PERCENT: i32 = 15;

pub static TREE: LazyLock<Tree> =
    LazyLock::new(|| serde_json::from_str(TREE_JSON).expect("tree json must be valid"));

pub static COSTED_JSON: LazyLock<String> = LazyLock::new(|| {
    let mut tree: serde_json::Value = serde_json::from_str(TREE_JSON).expect("tree json must be valid");
    for node in tree["nodes"].as_array_mut().into_iter().flatten() {
        if let Some(priced) = node["id"].as_u64().and_then(|id| TREE.node(id as u32)) {
            node["cost"] = priced.cost().into();
        }
    }
    tree.to_string()
});

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service((progress::unlock_item, progress::purchase_item, progress::skip_sticker));
}

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
    pub name: String,
    #[serde(default)]
    pub price: i32,
    pub kind: Kind,
    pub category: Option<String>,
    pub requires: Vec<u32>,
}

impl Node {
    pub fn cost(&self) -> i32 {
        if self.kind == Kind::Item && !tutorial(self.id) {
            (self.price * (100 - DISCOUNT_PERCENT) + 50) / 100
        } else {
            self.price
        }
    }
}

#[derive(Deserialize)]
pub struct Edge {
    pub from: u32,
    pub to: u32,
}

#[derive(Deserialize)]
pub struct Tree {
    pub currency: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Tree {
    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn is(&self, id: u32, kind: Kind) -> bool {
        self.node(id).is_some_and(|node| node.kind == kind)
    }
}
