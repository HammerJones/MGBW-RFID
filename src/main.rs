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
use std::io;

fn cap_input() -> String {

    let mut output = String::new();
    
    io::stdin().read_line(&mut output).expect("Failed to Capture User Input");

    output.trim().to_string()
}

fn write_new_id() {
    //Input first name
    println!("Please Input Employee's First Name: ");
    let first = cap_input();
    
    //Input last name
    println!("Please Input Employee's Last Name: ");
    let last = cap_input();

    //Input Boat
    println!("Please Input Employee's Current Boat Name: ");
    let boat = cap_input();

    //Input Task Number
    println!("Please Input Employee's Task Number: ");
    let task = cap_input();

    print!("{} {}: {} - {}", first, last, boat, task);
}
fn main() {
    write_new_id();
}
