//! Tests for Jump List data model.

use super::{JumpListTask, build_jump_list_tasks};

#[test]
fn build_returns_one_built_in_task() {
    let tasks = build_jump_list_tasks();
    assert_eq!(tasks.len(), 1);
}

#[test]
fn new_window_task_has_correct_arguments() {
    let tasks = build_jump_list_tasks();
    assert_eq!(tasks[0].label, "New Window");
    assert_eq!(tasks[0].arguments, "--new-window");
}

#[test]
fn task_labels_are_nonempty() {
    for task in &build_jump_list_tasks() {
        assert!(!task.label.is_empty(), "label should not be empty");
    }
}

#[test]
fn task_descriptions_are_nonempty() {
    for task in &build_jump_list_tasks() {
        assert!(
            !task.description.is_empty(),
            "description should not be empty for {}",
            task.label,
        );
    }
}

#[test]
fn task_arguments_use_hyphens_not_underscores() {
    for task in &build_jump_list_tasks() {
        assert!(
            !task.arguments.contains('_'),
            "argument {:?} should use hyphens, not underscores (CLI convention)",
            task.arguments,
        );
    }
}

#[test]
fn task_arguments_start_with_double_dash() {
    for task in &build_jump_list_tasks() {
        assert!(
            task.arguments.starts_with("--"),
            "argument {:?} should start with -- (long flag format)",
            task.arguments,
        );
    }
}

#[test]
fn jump_list_task_fields_are_populated() {
    let task = JumpListTask {
        label: "Test".to_owned(),
        arguments: "--test".to_owned(),
        description: "A test task".to_owned(),
    };
    assert_eq!(task.label, "Test");
    assert_eq!(task.arguments, "--test");
    assert_eq!(task.description, "A test task");
}
