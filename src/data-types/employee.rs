// This file holds all functions and vecs for Employee data

/* 
    Employee Class:
        - Employee name [First, Last]
        - Employee ID number
        - Date employed
        - Current Job Name & Task Number
        - Equipment checked out
        - Material Checkout history
        - Job/Task History

    Employee Class Functions:
        - Capture name
        - Assign ID Number
        - Apply date of employment
        - Archive prev. task number & Assign current task number
        - Assign equipment
        - Check out material & archive
*/
#[Path = "./data-types/equipment.rs"]
use Equipment::*;

enum Employee {
    Name { first: String, last: String },
    EmployeeID(String),
    EmploymentDate(String),
    //CurrentJob(???) <- Can I make this a task number? Will having the task enum having employee enums assigned to it create a circular dependency if the employee also stores which task they're assigned to?
    //JobHistory(???) <- How can I store a section of the log inside the employee object?
    EquipmentList(vec<Equipment>)
    //MaterialHistory(???) <- How can I store a section of the log inside the employee object?
}