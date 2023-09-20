use super::model::Task;
use serde::{Deserialize, Serialize};
use std::collections::{btree_set::BTreeSet, BTreeMap};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum PlannerError {
    #[error("cyclic dependencies found at task {0}")]
    CyclicDependencies(usize),
    #[error("dependent task {0} does not exist")]
    DependentTaskDoesNotExist(String),
    #[error("output {0} not unique")]
    OutputNotUnique(String),
    #[error("input ref {1} for task {0} not from a parent task")]
    OutputNotFromParent(String, String),
    #[error("input ref {1} for task {0} does not exist")]
    OutputDoesNotExist(String, String),
}

#[derive(PartialEq, Debug)]
pub(crate) struct Node {
    pub children: BTreeSet<usize>,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Plan(pub Vec<BTreeSet<usize>>);

fn construct_task_id_map(tasks: &[Task]) -> BTreeMap<&String, usize> {
    let mut task_id_map: BTreeMap<&String, usize> = BTreeMap::new();

    for (index, task) in tasks.iter().enumerate() {
        task_id_map.insert(&task.name, index);
    }

    task_id_map
}

fn construct_nodes(tasks: &[Task]) -> Result<Vec<Node>, PlannerError> {
    let task_id_map = construct_task_id_map(tasks);

    let mut nodes: Vec<Node> = vec![];

    for task in tasks.iter() {
        let mut node = Node {
            children: BTreeSet::new(),
        };

        for dep in task.depends.iter() {
            let child_node_id = match task_id_map.get(&dep) {
                None => return Err(PlannerError::DependentTaskDoesNotExist(dep.clone())),
                Some(id) => *id,
            };

            node.children.insert(child_node_id);
        }

        nodes.push(node);
    }

    Ok(nodes)
}

fn is_cyclic_visit(
    nodes: &Vec<Node>,
    node_id: usize,
    node: &Node,
    discovered: &mut BTreeSet<usize>,
    finished: &mut BTreeSet<usize>,
) -> Option<usize> {
    discovered.insert(node_id);

    for v in node.children.iter() {
        if discovered.contains(v) {
            return Some(*v);
        }

        if !finished.contains(v) {
            match is_cyclic_visit(nodes, *v, &nodes[*v], discovered, finished) {
                None => {
                    continue;
                }
                Some(v) => {
                    return Some(v);
                }
            }
        }
    }

    discovered.remove(&node_id);
    finished.insert(node_id);

    None
}

fn is_cyclic(nodes: &Vec<Node>) -> Option<usize> {
    let mut discovered = BTreeSet::new();
    let mut finished = BTreeSet::new();

    for (node_id, node) in nodes.iter().enumerate() {
        if !discovered.contains(&node_id) && !finished.contains(&node_id) {
            match is_cyclic_visit(nodes, node_id, node, &mut discovered, &mut finished) {
                None => {
                    continue;
                }
                Some(v) => {
                    return Some(v);
                }
            }
        }
    }

    None
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

    false
}

fn node_depends_on_stage(node: &Node, stage: &BTreeSet<usize>, nodes: &Vec<Node>) -> bool {
    for stage_node_id in stage {
        if node_depends_on_node(node, *stage_node_id, nodes) {
            return true;
        }
    }

    false
}

fn stage_depends_on_node(node_id: usize, stage: &BTreeSet<usize>, nodes: &Vec<Node>) -> bool {
    for stage_node_id in stage {
        let stage_node = &nodes[*stage_node_id];

        if node_depends_on_node(stage_node, node_id, nodes) {
            return true;
        }
    }

    false
}

fn add_node_to_plan(
    node_id: usize,
    node: &Node,
    plan: &mut Vec<BTreeSet<usize>>,
    nodes: &Vec<Node>,
) {
    for (stage_index, stage) in plan.iter_mut().enumerate() {
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

fn valid_input_outputs(tasks: &[Task], nodes: &[Node]) -> Result<(), PlannerError> {
    let mut output_task_name_map: BTreeMap<&String, usize> = BTreeMap::new();

    for (task_id, task) in tasks.iter().enumerate() {
        for outputs in &task.outputs {
            for output in outputs {
                if output_task_name_map.insert(&output.name, task_id).is_some() {
                    return Err(PlannerError::OutputNotUnique(output.name.clone()));
                }
            }
        }
    }

    for (task_id, task) in tasks.iter().enumerate() {
        for inputs in &task.inputs {
            for input in inputs {
                let Some(from_task_id) = output_task_name_map.get(&input.from) else {
                    return Err(PlannerError::OutputDoesNotExist(
                        task.name.clone(),
                        input.from.clone(),
                    ));
                };

                if !nodes[task_id].children.contains(from_task_id) {
                    return Err(PlannerError::OutputNotFromParent(
                        task.name.clone(),
                        input.from.clone(),
                    ));
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn construct_plan(tasks: &[Task]) -> Result<Plan, PlannerError> {
    let nodes = construct_nodes(tasks)?;

    if let Some(node_id) = is_cyclic(&nodes) {
        return Err(PlannerError::CyclicDependencies(node_id));
    }

    valid_input_outputs(tasks, &nodes)?;

    let mut stages: Vec<BTreeSet<usize>> = vec![];

    for (node_id, node) in nodes.iter().enumerate() {
        add_node_to_plan(node_id, node, &mut stages, &nodes);
    }

    Ok(Plan(stages))
}

#[cfg(test)]
mod tests {
    use crate::server::model::{Input, Output};

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

        assert_eq!(is_cyclic(&test_acyclic_nodes), None);
        assert_eq!(is_cyclic(&test_cyclic_nodes), Some(0));
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

        let expected_plan = Ok(Plan(vec![
            BTreeSet::from([0]),
            BTreeSet::from([3]),
            BTreeSet::from([1, 4]),
            BTreeSet::from([2]),
        ]));

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

        let expected = Err(PlannerError::OutputNotUnique("foo".to_owned()));
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

        let expected = Err(PlannerError::OutputDoesNotExist(
            "B".to_owned(),
            "doesNotExist".to_owned(),
        ));
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

        let expected = Err(PlannerError::OutputNotFromParent(
            "C".to_owned(),
            "foo".to_owned(),
        ));
        assert_eq!(actual, expected);
    }
}
