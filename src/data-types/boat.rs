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
#[path="./data-types/employee.rs"]
use Employee::*;

enum Boat {
    Name(String),
    TaskNumbers(vec<String>),
    Employees(vec<Employee>)
}