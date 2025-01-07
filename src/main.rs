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

//use k_board::{keyboard::Keyboard, keys::Keys};

enum WeldingMachine {
    miller_xmt(HashMap<String, String>),
    miller_suitcase(HashMap<String, String>),
    powermax_plasma(HashMap<String, String>),
    dynasty_tig(HashMap<String, String>),
}

enum WeldingGun {
    mig_gun(HashMap<String, String>),
    spool_gun(HashMap<String, String>),
    tig_gun(HashMap<String, String>),
    plasma_gun(HashMap<String, String>),
}

enum Tooling {
    flow_meter(HashMap<String, String>),
    chain_fall(HashMap<String, String>),
    come_along(HashMap<String, String>),
    vacuum(HashMap<String, String>),
}

enum Category {
    welding_machine(Option<WeldingMachine>),
    welding_gun(WeldingGun),
    tooling(Tooling),
}

struct EquipmentType {
    category: Option<Category>,
    purchase_date: String,
    repair_history: Vec<String>,
}

impl EquipmentType {
    fn new(category: String, name: String) -> Self {
        let output: EquipmentType;


        match category {
            _ => None,
            String::from("Welding Machine") => output.category = EquipmentType::
        }
        
        EquipmentType {
            category: None,
            purchase_date: String::new(),
            repair_history: Vec::new(),
        }
    }

    fn init(&mut self, category: String, name: String) {
        
    }
}

fn main() {
    
}
