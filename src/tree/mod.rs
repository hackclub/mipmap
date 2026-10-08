use actix_web::web;
use serde::Deserialize;
use std::sync::LazyLock;

mod images;
mod layout;
mod progress;

pub use images::{item_image, spawn as spawn_images};
pub use layout::LAYOUT_JSON;
pub use progress::{backfill_prices, refund, unlocked};

pub const TREE_JSON: &str = include_str!("../../Tree/Mipmap Tree.json");

pub static TREE: LazyLock<Tree> =
    LazyLock::new(|| serde_json::from_str(TREE_JSON).expect("tree json must be valid"));

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service((progress::unlock_item, progress::purchase_item));
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
