use super::model::Task;
use std::collections::{btree_set::BTreeSet, BTreeMap};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum PlannerError {
    #[error("cyclic depdencies found")]
    CyclicDependenciesError,
    #[error("dependent task does not exist")]
    DependentTaskDoesNotExistError,
    #[error("output not unique")]
    OutputNotUniqueError,
    #[error("output not from parent")]
    OutputNotFromParentError,
    #[error("output does not exist")]
    OutputDoesNotExistError,
}


#[derive(PartialEq, Debug)]
pub struct Node {
    pub children: BTreeSet<usize>,
}

fn construct_task_id_map(tasks: &Vec<Task>) -> BTreeMap<&String, usize> {
    let mut task_id_map: BTreeMap<&String, usize> = BTreeMap::new();

    for (index, task) in tasks.iter().enumerate() {
        task_id_map.insert(&task.name, index);
    }

    return task_id_map;
}

fn construct_nodes(tasks: &Vec<Task>) -> Result<Vec<Node>, PlannerError> {
    let task_id_map = construct_task_id_map(tasks);

    let mut nodes: Vec<Node> = vec![];

    for task in tasks.iter() {
        let mut node = Node {
            children: BTreeSet::new(),
        };

        for dep in task.depends.iter() {
            let child_node_id = match task_id_map.get(&dep) {
                None => return Err(PlannerError::DependentTaskDoesNotExistError),
                Some(id) => *id,
            };

            node.children.insert(child_node_id);
        }

        nodes.push(node);
    }

    return Ok(nodes);
}

fn is_cyclic_visit(
    nodes: &Vec<Node>,
    node_id: usize,
    node: &Node,
    discovered: &mut BTreeSet<usize>,
    finished: &mut BTreeSet<usize>,
) -> bool {
    discovered.insert(node_id);

    for v in node.children.iter() {
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

fn is_cyclic(nodes: &Vec<Node>) -> bool {
    let mut discovered = BTreeSet::new();
    let mut finished = BTreeSet::new();

    for (node_id, node) in nodes.iter().enumerate() {
        if !discovered.contains(&node_id) && !finished.contains(&node_id) {
            if is_cyclic_visit(nodes, node_id, node, &mut discovered, &mut finished) {
                return true;
            }
        }
    }

    return false;
}

fn node_depends_on_node(dependent: &Node, dependee_id: usize, nodes: &Vec<Node>) -> bool {
    if dependent.children.contains(&dependee_id) {
        return true;
    }

    for child_node_id in dependent.children.iter() {
        let child_of_dependent = &nodes[*child_node_id];

        if node_depends_on_node(child_of_dependent, dependee_id, nodes) {
            return true;
        }
    }

    return false;
}

fn node_depends_on_stage(node: &Node, stage: &BTreeSet<usize>, nodes: &Vec<Node>) -> bool {
    for stage_node_id in stage {
        if node_depends_on_node(node, *stage_node_id, nodes) {
            return true;
        }
    }

    return false;
}

fn stage_depends_on_node(node_id: usize, stage: &BTreeSet<usize>, nodes: &Vec<Node>) -> bool {
    for stage_node_id in stage {
        let stage_node = &nodes[*stage_node_id];

        if node_depends_on_node(stage_node, node_id, nodes) {
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
            continue;
        } else if stage_depends_on_node(node_id, stage, nodes) {
            plan.insert(stage_index, BTreeSet::from([node_id]));
            return;
        } else {
            stage.insert(node_id);
            return;
        }
    }

    plan.push(BTreeSet::from([node_id]));
}

fn valid_input_outputs(tasks: &Vec<Task>, nodes: &Vec<Node>) -> Result<(), PlannerError> {
    let mut output_task_name_map: BTreeMap<&String, usize> = BTreeMap::new();

    for (task_id, task) in tasks.iter().enumerate() {
        for outputs in &task.outputs {
            for output in outputs {
                if let Some(_) = output_task_name_map.insert(&output.name, task_id) {
                    return Err(PlannerError::OutputNotUniqueError);
                }
            }
        }
    }

    for (task_id, task) in tasks.iter().enumerate() {
        for inputs in &task.inputs {
            for input in inputs {
                let Some(from_task_id) = output_task_name_map.get(&input.from) else {
                    return Err(PlannerError::OutputDoesNotExistError);
                };

                if !nodes[task_id].children.contains(from_task_id) {
                    return Err(PlannerError::OutputNotFromParentError);
                }
            }
        }
    }

    return Ok(());
}

pub fn construct_plan(tasks: &Vec<Task>) -> Result<Vec<BTreeSet<usize>>, PlannerError> {
    let nodes = construct_nodes(tasks)?;

    if is_cyclic(&nodes) {
        return Err(PlannerError::CyclicDependenciesError);
    }

    valid_input_outputs(tasks, &nodes)?;

    let mut plan: Vec<BTreeSet<usize>> = vec![];

    for (node_id, node) in nodes.iter().enumerate() {
        add_node_to_plan(node_id, node, &mut plan, &nodes);
    }

    return Ok(plan);
}

#[cfg(test)]
mod tests {
    use crate::flow::model::{Input, Output};

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

    fn test_tasks() -> Vec<Task> {
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
        let nodes = construct_nodes(&test_tasks);

        let expected_nodes = Ok(vec![
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
        ]);

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

    #[test]
    fn test_output_not_unique() {
        let test_tasks = vec![
            Task {
                name: "A".to_string(),
                image: "".to_string(),
                depends: vec![],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: Some(vec![Output {
                    name: "foo".to_string(),
                    path: "/home/foo".to_string(),
                }]),
            },
            Task {
                name: "B".to_string(),
                image: "".to_string(),
                depends: vec!["A".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: Some(vec![Output {
                    name: "bar".to_string(),
                    path: "/home/bar".to_string(),
                }]),
            },
            Task {
                name: "C".to_string(),
                image: "".to_string(),
                depends: vec!["B".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: Some(vec![
                    Output {
                        name: "foo".to_string(),
                        path: "/home/foo".to_string(),
                    },
                    Output {
                        name: "alice".to_string(),
                        path: "/home/alice".to_string(),
                    },
                ]),
            },
        ];

        let actual = construct_plan(&test_tasks);

        let expected = Err(PlannerError::OutputNotUniqueError);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_output_does_not_exist() {
        let test_tasks = vec![
            Task {
                name: "A".to_string(),
                image: "".to_string(),
                depends: vec![],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: Some(vec![Output {
                    name: "foo".to_string(),
                    path: "/home/foo".to_string(),
                }]),
            },
            Task {
                name: "B".to_string(),
                image: "".to_string(),
                depends: vec!["A".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: Some(vec![Input {
                    from: "doesNotExist".to_string(),
                    path: "/user/doesNotExist".to_string(),
                }]),
                outputs: Some(vec![Output {
                    name: "bar".to_string(),
                    path: "/home/bar".to_string(),
                }]),
            },
        ];

        let actual = construct_plan(&test_tasks);

        let expected = Err(PlannerError::OutputDoesNotExistError);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_output_not_from_parent() {
        let test_tasks = vec![
            Task {
                name: "A".to_string(),
                image: "".to_string(),
                depends: vec![],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: Some(vec![Output {
                    name: "foo".to_string(),
                    path: "/home/foo".to_string(),
                }]),
            },
            Task {
                name: "B".to_string(),
                image: "".to_string(),
                depends: vec!["A".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: None,
                outputs: Some(vec![Output {
                    name: "bar".to_string(),
                    path: "/home/bar".to_string(),
                }]),
            },
            Task {
                name: "C".to_string(),
                image: "".to_string(),
                depends: vec!["B".to_string()],
                cmd: vec![],
                env: vec![],
                inputs: Some(vec![
                    Input {
                        from: "foo".to_string(),
                        path: "/user/foo".to_string(),
                    },
                    Input {
                        from: "bae".to_string(),
                        path: "/user/bar".to_string(),
                    },
                ]),
                outputs: Some(vec![Output {
                    name: "alice".to_string(),
                    path: "/home/alice".to_string(),
                }]),
            },
        ];

        let actual = construct_plan(&test_tasks);

        let expected = Err(PlannerError::OutputNotFromParentError);
        assert_eq!(actual, expected);
    }
}
