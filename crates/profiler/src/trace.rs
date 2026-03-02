use std::time::Instant;

use crate::render::render_trace;

pub(crate) struct ActiveTrace {
    pub(crate) nodes: Vec<Node>,
    pub(crate) stack: Vec<Frame>,
    pub(crate) emit: bool,
    pub(crate) pretty: bool,
    pub(crate) command: Vec<u8>,
    pub(crate) key: Option<Vec<u8>>,
}

impl ActiveTrace {
    pub(crate) fn new(command: Vec<u8>, pretty: bool) -> Self {
        let mut nodes = Vec::with_capacity(32);
        nodes.push(Node::new("request"));
        Self {
            nodes,
            stack: vec![Frame::new(0)],
            emit: true,
            pretty,
            command,
            key: None,
        }
    }

    pub(crate) fn enter_scope(&mut self, name: &'static str) {
        let parent_index = match self.stack.last() {
            Some(frame) => frame.node_index,
            None => return,
        };
        let node_index = self.nodes.len();
        self.nodes.push(Node::new(name));
        self.nodes[parent_index].children.push(node_index);
        self.stack.push(Frame::new(node_index));
    }

    pub(crate) fn exit_scope(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }
        self.close_top_frame();
    }

    pub(crate) fn close_all_scopes(&mut self) {
        while !self.stack.is_empty() {
            self.close_top_frame();
        }
    }

    fn close_top_frame(&mut self) {
        let Some(frame) = self.stack.pop() else {
            return;
        };
        let total_ns = elapsed_ns(frame.started);
        let self_ns = total_ns.saturating_sub(frame.child_total_ns);
        self.nodes[frame.node_index].total_ns = total_ns;
        self.nodes[frame.node_index].self_ns = self_ns;
        if let Some(parent) = self.stack.last_mut() {
            parent.child_total_ns = parent.child_total_ns.saturating_add(total_ns);
        }
    }

    pub(crate) fn emit(&self) {
        render_trace(self);
    }
}

pub(crate) struct Node {
    pub(crate) name: &'static str,
    pub(crate) total_ns: u64,
    pub(crate) self_ns: u64,
    pub(crate) children: Vec<usize>,
}

impl Node {
    pub(crate) fn new(name: &'static str) -> Self {
        Self {
            name,
            total_ns: 0,
            self_ns: 0,
            children: Vec::new(),
        }
    }
}

pub(crate) struct Frame {
    pub(crate) node_index: usize,
    pub(crate) started: Instant,
    pub(crate) child_total_ns: u64,
}

impl Frame {
    pub(crate) fn new(node_index: usize) -> Self {
        Self {
            node_index,
            started: Instant::now(),
            child_total_ns: 0,
        }
    }
}

fn elapsed_ns(started: Instant) -> u64 {
    started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
}
