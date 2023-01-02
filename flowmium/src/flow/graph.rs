use std::collections::btree_set::BTreeSet;

#[derive(PartialEq, Debug)]
pub struct Node {
    pub children: BTreeSet<usize>,
}

pub fn is_cyclic(nodes: &Vec<Node>) -> bool {
    let mut discovered = BTreeSet::new();
    let mut finished = BTreeSet::new();

    for (index, node) in nodes.iter().enumerate() {
        if !discovered.contains(&index) && !finished.contains(&index) {
            if is_cyclic_visit(nodes, index, node, &mut discovered, &mut finished) {
                return true;
            }
        }
    }

    return false;
}

fn is_cyclic_visit(
    nodes: &Vec<Node>,
    node_id: usize,
    node: &Node,
    discovered: &mut BTreeSet<usize>,
    finished: &mut BTreeSet<usize>,
) -> bool {
    discovered.insert(node_id);

    for v in &node.children {
        if discovered.contains(&v) {
            return true;
        }

        if !finished.contains(&v) {
            if is_cyclic_visit(nodes, *v, &nodes[*v], discovered, finished) {
                return true;
            }
        }
    }

    discovered.remove(&node_id);
    finished.insert(node_id);

    return false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cyclic() {
        let test_acyclic_nodes = vec![
            Node {
                children: BTreeSet::from([1, 2, 3, 4]),
            },
            Node {
                children: BTreeSet::from([3]),
            },
            Node {
                children: BTreeSet::from([3]),
            },
            Node {
                children: BTreeSet::from([4]),
            },
            Node {
                children: BTreeSet::new(),
            },
        ];

        let test_cyclic_nodes = vec![
            Node {
                children: BTreeSet::from([1, 2, 3, 4]),
            },
            Node {
                children: BTreeSet::from([3]),
            },
            Node {
                children: BTreeSet::from([1]),
            },
            Node {
                children: BTreeSet::from([0]),
            },
            Node {
                children: BTreeSet::new(),
            },
        ];

        assert_eq!(is_cyclic(&test_acyclic_nodes), false);
        assert_eq!(is_cyclic(&test_cyclic_nodes), true);
    }
}
