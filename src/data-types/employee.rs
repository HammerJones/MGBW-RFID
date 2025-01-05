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
#[path = "./equipment.rs"]
mod equipment;
use equipment::Equipment;

pub struct EmployeeName {
    pub first: String,
    pub last: String,
}
pub struct Employee {
    pub name: EmployeeName,
    pub employee_id: String,
    pub employment_date: String,
    //CurrentJob(???) <- Can I make this a task number? Will having the task enum having employee enums assigned to it create a circular dependency if the employee also stores which task they're assigned to?
    //JobHistory(???) <- How can I store a section of the log inside the employee object?
    pub equipment_list: Vec<Equipment>
    //MaterialHistory(???) <- How can I store a section of the log inside the employee object?
}