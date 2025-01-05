#[path="./input.rs"]
mod input;
use input::*;

pub struct Task {
    pub task_number: String,
    pub department: String,
}

pub struct Boat {
    pub boat_name: String,
    pub task: Vec<Task>,
}

impl Task {
    pub fn new() -> Self {
        Task {
            task_number: String::new(),
            department: String::new(),
        }
    }
    
    pub fn init(init_task_number: String, init_department: String) -> Task {
        let init_task: Task = Task {
            task_number: init_task_number,
            department: init_department,
        };

        init_task
    }
}

impl Boat {
    pub fn new(init_name: String) -> Self {
        Boat { 
            boat_name: init_name, 
            task: Vec::new(), 
        }
    }

    pub fn add_task(&mut self, init_task: Task) {
        self.task.push(init_task)
    }

    pub fn query_user(&mut self) {
        print!("Please enter boat name: ");
        clear_screen();
        self.boat_name = cap_input();

        print!("Please Enter Task Number then Task Department: ");
        clear_screen();
        self.add_task(
        Task::init(
            cap_input(),
            cap_input()));

        print!("You have made the following inputs:\n
        Boat: {}\n
        Task: {}\n
        Department: {}\n", 
        self.boat_name,
        self.task[self.task.len()-1].task_number,
        self.task[self.task.len()-1].department);
    }
}