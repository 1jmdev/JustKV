use super::node::Node;

pub struct Iter<'a, K, V> {
    pub(super) nodes: &'a [Option<Node<K, V>>],
    pub(super) index: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.nodes.len() {
            let idx = self.index;
            self.index += 1;
            if let Some(node) = self.nodes[idx].as_ref() {
                return Some((&node.key, &node.value));
            }
        }
        None
    }
}
