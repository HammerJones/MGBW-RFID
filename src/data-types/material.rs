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

enum Material {
    PartNumber(u32),
    VendorInfo(String),
    Description(String),
    //Departments(???),
    StockLevel(i32),
    StockSetPoint(i32),
    StockReorderPoint(i32),
}