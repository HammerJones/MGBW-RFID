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

#![allow(nonstandard_style)]
#![allow(dead_code)]
#![allow(special_module_name)]
#![allow(unused_imports)]

use std::collections::HashMap;
mod menu;
use crate::menu::input::{cap_input, clear_screen};

//use k_board::{keyboard::Keyboard, keys::Keys};


 pub struct Equipment {
    pub line_item: HashMap<String, String>,
    pub menu_item: HashMap<i32, String>,
}

impl Equipment {
    pub fn new() -> Self {
        Equipment {
            line_item: HashMap::new(),
            menu_item: HashMap::new(),
        }
    }
    pub fn create_new_field(&mut self) {
        let input1: String;
        let input2: String;

        print!("Please enter field name: ");
        clear_screen();
        input1 = cap_input();

        print!("Please enter field value: ");
        clear_screen();
        input2 = cap_input();

        self.line_item.insert(input1, input2);
    }
    pub fn query_user(&mut self) {
        print!("Would you like to create a new field?\n[y/n]\n");
        clear_screen();
        while cap_input() == "y" {
            self.create_new_field();
            print!("Would you like to create a new field?\n[y/n]\n");
        }
        for (key, value) in self.line_item.clone().into_iter() {
            print!("{} - {}\n", key, value);
            clear_screen()
        }
    }
}

fn main() {
    let mut xmt1: Equipment = Equipment::new();
    xmt1.query_user();
}
