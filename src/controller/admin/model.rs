
pub struct AdminCat {
    pub id: i64,
    pub name: String,
    pub sort: i64,
    pub image_count: i64,
}

pub struct AdminImage {
    pub id: i64,
    pub title: String,
    pub category_name: String,
    pub model_name: String,
    pub price_text: String,
    pub is_published: bool,
    pub thumb: String,
    pub created_at: String,
}

pub struct AdminOrder {
    pub order_no: String,
    pub image_title: String,
    pub amount_text: String,
    pub status: String,
    pub pay_type: String,
    pub created_at: String,
    pub paid_at: String,
}

pub struct SelectCat {
    pub id: i64,
    pub name: String,
}

pub struct ImageForm {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub category_id: i64,
    pub model_name: String,
    pub width: i64,
    pub height: i64,
    pub file_path: String,
    pub prompt: String,
    pub price_yuan: String,
    pub is_published: bool,
    pub is_new: bool,
}