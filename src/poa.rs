use crate::node::Node;

pub struct PoA {
    pub authorities: Vec<Node>,
}

impl PoA {
    pub fn new() -> Self {
        PoA { authorities: vec![] }
    }

    pub fn add_authority(&mut self, node: Node) {
        self.authorities.push(node);
    }

    pub fn is_authority(&self, node_id: &str) -> bool {
        self.authorities.iter().any(|node| node.id == node_id && node.is_authority)
    }
}
