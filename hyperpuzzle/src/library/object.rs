#[derive(Debug, Clone)]
pub struct Object {
    pub name: String,
    pub id: String,
    pub data: ObjectData,
}

#[derive(Debug, Clone)]
pub enum ObjectData {
    Puzzle { ndim: u8 },
}
