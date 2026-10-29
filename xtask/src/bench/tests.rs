use super::*;

#[test]
fn counts_add_up_every_test_result_line() {
    let output = "test result: ok. 3 passed; 0 failed; 0 ignored\n\
                  test result: FAILED. 1 passed; 2 failed; 0 ignored\n";
    assert_eq!(counts(output), (4, 2));
}

#[test]
fn every_task_has_a_prompt_and_acceptance_tests() {
    let root = repo_root();
    let tasks = tasks(&root).unwrap();
    assert!(tasks.len() >= 4, "{tasks:?}");
    for task in tasks {
        let dir = root.join("tests/agent/tasks").join(&task);
        let prompt = std::fs::read_to_string(dir.join("TASK.md")).unwrap();
        assert!(
            prompt.contains("{{engine}}"),
            "{task}: TASK.md never names the engine"
        );
        assert!(
            dir.join("acceptance").read_dir().unwrap().next().is_some(),
            "{task}"
        );
    }
}

#[test]
fn every_starter_belongs_to_a_task_with_its_source() {
    let root = repo_root();
    for (task, _) in STARTERS {
        assert!(root
            .join("tests/agent/tasks")
            .join(task)
            .join("starter/lib.rs")
            .is_file());
    }
}
