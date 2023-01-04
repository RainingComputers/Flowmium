use std::collections::{btree_set::BTreeSet, BTreeMap};

use super::graph::{is_cyclic, Node};
use super::model::{ContainerDAGFlow, Task};

#[derive(Debug, PartialEq)]
enum ExecutorError {
    CyclicDependenciesError,
}

fn construct_task_id_map(tasks: &Vec<Task>) -> BTreeMap<&str, usize> {
    let mut task_id_map: BTreeMap<&str, usize> = BTreeMap::new();

    for (index, task) in tasks.iter().enumerate() {
        task_id_map.insert(&task.name, index);
    }

    return task_id_map;
}

fn construct_nodes(tasks: &Vec<Task>, task_id_map: &BTreeMap<&str, usize>) -> Vec<Node> {
    // TODO clean up and remove [] indexing

    let mut nodes: Vec<Node> = tasks
        .iter()
        .map(|_| Node {
            children: BTreeSet::new(),
        })
        .collect();

    for (index, task) in tasks.iter().enumerate() {
        for dep in &task.depends {
            nodes[index].children.insert(task_id_map[&dep[..]]);
        }
    }

    return nodes;
}

fn node_depends_on_stage(node: &Node, stage: &BTreeSet<usize>, nodes: &Vec<Node>) -> bool {
    if node.children.is_disjoint(stage) {
        for child_node_id in &node.children {
            let child_node = &nodes[*child_node_id]; // TODO: Prove that [] indexing is okay and wont error
            return node_depends_on_stage(&child_node, stage, nodes);
        }

        return false;
    }

    return true;
}

fn stage_depends_on_node(node_id: usize, stage: &BTreeSet<usize>, nodes: &Vec<Node>) -> bool {
    for stage_node_id in stage {
        let stage_node = &nodes[*stage_node_id]; // TODO: Prove that [] indexing is okay and wont error

        if stage_node.children.contains(&node_id) {
            return true;
        }
    }

    return false;
}

fn add_node_to_plan(
    node_id: usize,
    node: &Node,
    plan: &mut Vec<BTreeSet<usize>>,
    nodes: &Vec<Node>,
) {
    for (stage_index, stage) in plan.into_iter().enumerate() {
        if node_depends_on_stage(node, stage, nodes) {
            println!("Node {} depends on stage {}", node_id, stage_index);
            continue;
        } else if stage_depends_on_node(node_id, stage, nodes) {
            println!("Stage {} depends on node {}", stage_index, node_id);
            plan.insert(stage_index, BTreeSet::from([node_id]));
            return;
        } else {
            println!("Adding node {} to existing stage {}", stage_index, node_id);
            stage.insert(node_id);
            return;
        }
    }

    println!("Creating new stage for {}", node_id);
    plan.push(BTreeSet::from([node_id]));
}

fn construct_plan(tasks: &Vec<Task>) -> Result<Vec<BTreeSet<usize>>, ExecutorError> {
    let task_id_map = construct_task_id_map(tasks);
    let nodes = construct_nodes(tasks, &task_id_map);

    println!("The nodes are {:?}", nodes);

    if is_cyclic(&nodes) {
        return Err(ExecutorError::CyclicDependenciesError);
    }

    let mut plan: Vec<BTreeSet<usize>> = vec![];

    for (node_id, node) in nodes.iter().enumerate() {
        add_node_to_plan(node_id, node, &mut plan, &nodes);
        println!("The plan is {:?}", plan);
    }

    return Ok(plan);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tasks() -> Vec<Task> {
        // TODO: More complex example, remove println! statements
        vec![
            Task {
                name: "E".to_string(),
                image: "".to_string(),
                depends: vec![],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "B".to_string(),
                image: "".to_string(),
                depends: vec!["D".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "A".to_string(),
                image: "".to_string(),
                depends: vec![
                    "B".to_string(),
                    "C".to_string(),
                    "D".to_string(),
                    "E".to_string(),
                ],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "D".to_string(),
                image: "".to_string(),
                depends: vec!["E".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
            Task {
                name: "C".to_string(),
                image: "".to_string(),
                depends: vec!["D".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: None,
            },
        ]
    }

    #[test]
    fn test_construct_deps() {
        let test_tasks = test_tasks();
        let task_id_map = construct_task_id_map(&test_tasks);

        let nodes = construct_nodes(&test_tasks, &task_id_map);

        let expected_nodes = vec![
            Node {
                children: BTreeSet::new(),
            },
            Node {
                children: BTreeSet::from([3]),
            },
            Node {
                children: BTreeSet::from([0, 1, 3, 4]),
            },
            Node {
                children: BTreeSet::from([0]),
            },
            Node {
                children: BTreeSet::from([3]),
            },
        ];

        assert_eq!(nodes, expected_nodes);
    }

    #[test]
    fn test_construct_plan() {
        let test_tasks = test_tasks();

        let plan = construct_plan(&test_tasks);

        let expected_plan = Ok(vec![
            BTreeSet::from([0]),
            BTreeSet::from([3]),
            BTreeSet::from([1, 4]),
            BTreeSet::from([2]),
        ]);

        assert_eq!(plan, expected_plan);
    }
}
