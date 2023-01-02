use std::collections::btree_set::BTreeSet;

pub fn is_cyclic(nodes: &Vec<BTreeSet<usize>>) -> bool {
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
    nodes: &Vec<BTreeSet<usize>>,
    node_id: usize,
    node: &BTreeSet<usize>,
    discovered: &mut BTreeSet<usize>,
    finished: &mut BTreeSet<usize>,
) -> bool {
    discovered.insert(node_id);

    for v in node {
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
            BTreeSet::from([1, 2, 3, 4]),
            BTreeSet::from([3]),
            BTreeSet::from([3]),
            BTreeSet::from([4]),
            BTreeSet::new(),
        ];

        let test_cyclic_nodes = vec![
            BTreeSet::from([1, 2, 3, 4]),
            BTreeSet::from([3]),
            BTreeSet::from([1]),
            BTreeSet::from([0]),
            BTreeSet::new(),
        ];

        assert_eq!(is_cyclic(&test_acyclic_nodes), false);
        assert_eq!(is_cyclic(&test_cyclic_nodes), true);
    }
}
