use super::Node;

pub struct Header<Metadata> {
    pub next: Option<Node<Metadata>>,
    pub previous: Option<Node<Metadata>>,
    pub metadata: Metadata,
}
