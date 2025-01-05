// This file holds all functions and vecs for boats & task numbers
// The Boat Enum will act as the parent data structure for all other structures:
/*     - i.e. Boat
              |-> Task Number
                  |-> Employees
                      |-> Equipment
                      |-> Materials
*//*
    Boat Class:
        - Boat Name
        - Task Numbers assigned to boat
        - Employees assigned to task numbers
        - Equipment/Materials assigned to employees
*/
#[path="./employee.rs"]
mod employee;
use employee::Employee;

pub struct Boat {
    pub name: String,
    pub task_numbers: Vec<String>,
    pub employees: Vec<Employee>,
}