// This file will act as my main application,
// Step 1: Write basic CLI program that has 3 options
//      - [1] Write New ID
//          - Enter Employee Name: [First, Last Name]
//          - Enter Current Boat Name: [Boat Name]
//          - Enter Current Task Number: [Task Number]
//          - Print: "[Employee Name] has been assigned to [Boat Name] - [Task Number]"
//      - [2] Read ID
//          - Print: "[Employee Name]"
//                  "[Employee Boat Name] - [Employee Task Number]"
//              - Check In/Out Equipment [1]
//              - Check Out Materials [2]
//              - Exit [3]
//      - [3] Open Current Task Numbers
//          - Print: [x] - [Current List of Open Task Numbers]
//              - Enter Number you wish to view: [x]
//          - Print: [Selected Boat Name] - [Task Number]
//              - [1] - View Current Equipment
//              - [2] - View Current Invoices
//              - [3] - View Current Employees
//              - [4] - Exit 
mod lib;
use std::collections::btree_map::Iter;

use lib::*;

mod boat;
use boat::*;

mod input;
use input::*;

fn main() {
    print!("Please enter a boat name: ");
    clear_screen();
    let mut boat1: Boat = Boat::new(cap_input());
    
    boat1.query_user();
    boat1.query_user();
    boat1.query_user();

    let mut i = 0;
    for item in boat1.task {
        i += 1;

        print!("[{}] - {} - {}\n", 
        i,
        item.task_number, 
        item.department)
    }
}
