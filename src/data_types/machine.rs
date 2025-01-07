#![allow(dead_code)]
#![allow(unused_imports)]

use std::collections::HashMap;

#[path="./menu/input.rs"]
use crate::menu::input::*;
/*
enum Category {
    welding_machine(WeldingMachine),
}
enum WeldingMachine {
    miller_xmt(HashMap<String, String>),
    miller_suitcase(HashMap<String, String>),
    powermax_plasma(HashMap<String, String>),
    dynasty_tig(HashMap<String, String>),
}

pub struct Equipment {
    pub category: Category,
}
*/

/* 
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
*/
