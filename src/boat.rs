#![allow(dead_code)]
#![allow(unused_imports)]

use std::{collections::HashMap, hash::Hash};

#[path="./input.rs"]
mod input;

use input::*;

#[derive(Eq, Hash, PartialEq)]
pub struct Task {
    pub task_number: String,
    pub department: String,
}

pub struct Yard {
    pub active_tasks: HashMap<String, Task>,
}

impl Task {
    pub fn new() -> Self {
        Task {
            task_number: String::new(),
            department: String::new(),
        }
    }
    pub fn add_task(&self) -> Self {
        let mut new_task: Task = Task::new();

        print!("Please Enter Task Number: ");
        clear_screen();
        new_task.task_number = cap_input();

        print!("Hooray! You input: {}", new_task.task_number);
        clear_screen();

        new_task
    }
}

impl Yard {
    pub fn new() -> Self {
        Yard {
            active_tasks: HashMap::new(),
        }
    }
}