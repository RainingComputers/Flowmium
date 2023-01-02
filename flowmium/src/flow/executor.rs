use std::collections::{btree_set::BTreeSet, BTreeMap};

use super::graph::is_cyclic;
use super::model::{ContainerDAGFlow, Task};

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

fn construct_nodes(tasks: &Vec<Task>, task_id_map: &BTreeMap<&str, usize>) -> Vec<BTreeSet<usize>> {
    let mut nodes: Vec<BTreeSet<usize>> = tasks.iter().map(|_| BTreeSet::new()).collect();

    for (index, task) in tasks.iter().enumerate() {
        for dep in &task.depends {
            nodes[index].insert(task_id_map[&dep[..]]);
        }
    }

    return nodes;
}

fn get_plan(workflow: &ContainerDAGFlow) -> Result<(Vec<Vec<usize>>, &Vec<Task>), ExecutorError> {
    let task_id_map = construct_task_id_map(&workflow.tasks);
    let nodes = construct_nodes(&workflow.tasks, &task_id_map);

    if is_cyclic(&nodes) {
        return Err(ExecutorError::CyclicDependenciesError);
    }

    return Ok((vec![], &workflow.tasks));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tasks() -> Vec<Task> {
        // vec![
        //     Task {
        //         name: "E".to_string(),
        //         image: "".to_string(),
        //         depends: vec![],
        //         cmd: vec![],
        //         env: vec![],
        //         inputs: None,
        //         outputs: None,
        //     },
        //     Task {
        //         name: "B".to_string(),
        //         image: "".to_string(),
        //         depends: vec!["D".to_string()],
        //         cmd: vec![],
        //         env: vec![],
        //         inputs: None,
        //         outputs: None,
        //     },
        //     Task {
        //         name: "A".to_string(),
        //         image: "".to_string(),
        //         depends: vec![
        //             "B".to_string(),
        //             "C".to_string(),
        //             "D".to_string(),
        //             "E".to_string(),
        //         ],
        //         cmd: vec![],
        //         env: vec![],
        //         inputs: None,
        //         outputs: None,
        //     },
        //     Task {
        //         name: "D".to_string(),
        //         image: "".to_string(),
        //         depends: vec!["E".to_string()],
        //         cmd: vec![],
        //         env: vec![],
        //         inputs: None,
        //         outputs: None,
        //     },
        //     Task {
        //         name: "C".to_string(),
        //         image: "".to_string(),
        //         depends: vec!["D".to_string()],
        //         cmd: vec![],
        //         env: vec![],
        //         inputs: None,
        //         outputs: None,
        //     },
        // ]

        vec![
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
                name: "B".to_string(),
                image: "".to_string(),
                depends: vec!["D".to_string()],
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
                name: "E".to_string(),
                image: "".to_string(),
                depends: vec![],
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
            BTreeSet::from([1, 2, 3, 4]),
            BTreeSet::from([3]),
            BTreeSet::from([3]),
            BTreeSet::from([4]),
            BTreeSet::new(),
        ];

        assert_eq!(nodes, expected_nodes);
    }
}
