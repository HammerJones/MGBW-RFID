use std::collections::HashMap;

pub struct Menu {
    title: String,
    description: String,
    instructions: String,
    items: HashMap<i32, String>,
}