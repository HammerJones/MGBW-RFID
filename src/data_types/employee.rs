#![allow(dead_code)]
#![allow(unused_imports)]


use crate::data_types::machine::*;


pub struct Employee {
    first: String,
    last: String,
    department: String,
    id_number: i32,
    equipment: Vec<Equipment>,
    identifier: i32,
}