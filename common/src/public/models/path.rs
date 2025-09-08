use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct IdPath {
    id: String,
}

impl IdPath {
    pub fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Deserialize)]
pub struct IdPathTwo {
    pub id: String,
    pub id_2: String,
}

impl IdPathTwo {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn id_2(&self) -> &str {
        &self.id_2
    }
}

//isPathMont
#[derive(Deserialize)]
pub struct IdPathTwoMonth {
    id: String,
    month: String,
    id2: String,
}

impl IdPathTwoMonth {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn month(&self) -> &str {
        &self.month
    }

    pub fn id2(&self) -> &str {
        &self.id2
    }
}


#[derive(Deserialize)]
pub struct IdPathThreeRangeDates {
    pub id: String,
    pub id_2: String,
    pub start_date: String,
    pub end_date: String,
}

impl IdPathThreeRangeDates {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn id2(&self) -> &str {
        &self.id_2
    }

    pub fn start_date(&self) -> &str {
        &self.start_date
    }

    pub fn end_date(&self) -> &str {
        &self.end_date
    }
}
