#![allow(dead_code)]
#![allow(unused_imports)]


use crate::data_types::employee::*;
use crate::data_types::machine;

//use super::employee::Employee;


pub struct Task {
    task_number: String,
    task_staff: Vec<Employee>,
}

pub struct Boat {
    boat_name: String,
    active_task_numbers: Vec<Task>,
}