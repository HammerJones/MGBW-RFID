// The Equipment Enum will store any relevant enums/vecs for different equipment types

/*
    Equipment Enum:
        - Equipment Type (make/model)
        - Serial No. (required for all equipment, if not present, one will be generated for internal use)
        - Date Purchased
        - Repair & Maintenance history
        - Check Out History 
*/

enum Equipment {
    Type(String, String),
    Serial(String),
    PurchaseDate(String),
    RepairHistory(String),
}