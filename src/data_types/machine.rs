#![allow(dead_code)]
#![allow(unused_imports)]

use std::collections::HashMap;

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
        if cap_input() == "y" {
            self.create_new_field();
        } if cap_input() == "n" {
            for (key, value) in self.line_item.clone().into_iter() {
                print!("{} - {}", key, value);
                clear_screen();
            }
        }
    }
}