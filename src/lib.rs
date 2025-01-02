//This will be a collection of traits & functions to be used by data types

pub trait Initialize {
    fn init_object();
}
pub trait Archive {
    fn generate_entry() -> String;
    fn archive_entry();
}