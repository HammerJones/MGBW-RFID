// The Material Enum will store the relevant Enum & Functions/Vecs

/*
    Material Enum:
        - Part No.
        - Vendor Info.
        - Description (Includes color/shape & function)
        - Relevant Department(s)
        - Inventory Level
        - Inventory Set Point
        - Inventory Re order point 
*/

pub struct Material {
    part_number: String,
    vendor_info: String,
    description: String,
    //Departments(???),
    stock_level: i32,
    stock_set_point: i32,
    stock_reorder_point: i32,
}