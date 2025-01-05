use std::{io, net::Incoming};

#[path="./lib.rs"]
use mod lib;
use lib::Initialize;


pub struct Menu {
    title: String,
    description: String,
    instructions: String,
    items: Vec<String>,
}

trait Construct {
    fn add_menu_item(&mut self, item_name: String);
    fn construct(title_init: String, 
        description_init: String, 
        items_init: Vec<String>) 
        -> Menu;
    fn dsiplay(&self);
}

impl Initialize for Menu {
    fn new() -> Self {
        
        let new_menu = Menu{

            title: String::new(),
            description: String::new(),
            instructions: String::new(),
            items: Vec::new(),

        };

        new_menu
    }
    fn init_object() -> Self {
        Menu::new()
    }
}

impl Construct for Menu {
    fn add_menu_item(&mut self, item_name: String) {
        self.items.push(item_name);
    }
    fn construct(title_init: String, 
        description_init: String, 
        items_init: Vec<String>) -> Menu{
            
            Menu {

                title: String::from(title_init),
                description: String::from(description_init),
                instructions: String::from("Please input the number you wish to select then press enter.\n"),
                items: Vec::from(items_init),

            }

    }

    fn dsiplay(&self) {
        println!("{}", self.title);
        println!("{}", self.description);
        println!("{}", self.instructions);
        
        for item in &self.items {
            println!("[x] - {}", item);
        }
    }
}

fn add_menu_item(item_name: String) {

}

/* ----- Equipment Menu ----- */
pub fn equipment_category_menu() {
    println!("You are now adding a new piece of equipment to inventory.\n");
    println!("Please press key corresponding to equipment type:");
    println!("  [1] - Welding Machines");
    println!("  [2] - Welding Guns");
    println!("  [3] - Safety Equipment");
    println!("  [4] - General Equipment & Tooling");
    println!("  [5] - Electrical & Safety Lines");
    println!("  [6] - Ventilation");

    let selection = cap_input();

    //match selection {
    //    None => None,
    //}
}

pub fn equipment_init_menu() {

}

/* ----- General Functions ----- */
pub fn clear_screen() {
    print!("{}[2J", 27 as char);
}

pub fn cap_input() -> String {

    let mut output = String::new();
    
    io::stdin().read_line(&mut output).expect("Failed to Capture User Input");

    output.trim().to_string()
    
}

pub fn select_choice() {

}