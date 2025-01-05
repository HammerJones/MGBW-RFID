// This will be a collection of traits & functions to be used by data types

/* ----- File Dependencies ----- */
    #[path="./menu_templates.rs"]
    mod menu_templates;

    #[path="./data-types/equipment.rs"]
    mod equipment;
    use equipment::Equipment;

    #[path="./data-types/material.rs"]
    mod material;
    use material::Material;

    #[path="./data-types/employee.rs"]
    mod employee;
    use employee::Employee;

    #[path="./data-types/boat.rs"]
    mod boat;
    use boat::Boat;

/* ----- Traits ----- */
pub trait Initialize {
    fn new() -> Self;
    fn init_object() -> Self;
}

pub trait Archive {
    fn generate_entry() -> String;
    fn archive_entry();
}

/* ----- Implementations ----- */

impl Initialize for Equipment {
    fn new() -> Self {
        let new_equipment: Equipment = Equipment {
            category: String::new(),
            brand: String::new(),
            model: String::new(),
            serial: String::new(),
            purchase_date: String::new(),
            repair_history: String::new(),
        };

        new_equipment
    }
    fn init_object() -> Equipment {
        let init_equipment: Equipment = Equipment::new();

        menu_templates::clear_screen();
        menu_templates::equipment_category_menu();

        init_equipment
    }
}