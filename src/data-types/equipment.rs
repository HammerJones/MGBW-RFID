// The Equipment Enum will store any relevant enums/vecs for different equipment types

/*
    Equipment Enum:
        - Equipment Type (make/model)
        - Serial No. (required for all equipment, if not present, one will be generated for internal use)
        - Date Purchased
        - Repair & Maintenance history
        - Check Out History 
*/

//pub enum Equipment {
//    Type{category: String, make: String, model: String},
//    Serial(String),
//    PurchaseDate(String),
//    RepairHistory(String),
//}

pub struct Equipment {
    pub category: String,
    pub brand: String,
    pub model: String,
    pub serial: String,
    pub purchase_date: String,
    pub repair_history: String,
}